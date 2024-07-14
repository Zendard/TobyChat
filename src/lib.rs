use rocket::{
    form::{Form, FromForm},
    http::{Cookie, CookieJar, Status},
    post,
    request::{FromRequest, Outcome, Request},
    response::Redirect,
    serde::Serialize,
};
use serde::Deserialize;
use surrealdb::{
    engine::remote::ws::{self, Client},
    sql::{Datetime, Uuid},
    Surreal,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct User {
    email: String,
    username: String,
    rooms: Vec<Room>,
    created: Datetime,
}

#[derive(Debug)]
pub struct NotLoggedIn;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = NotLoggedIn;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session_token = match req.cookies().get_private("session") {
            Some(token) => token.value().to_owned(),
            None => return Outcome::Forward(Status::Unauthorized),
        };

        let db = connect_to_db().await;

        let response = db
            .query("(SELECT email,username,created FROM user WHERE session='$session_token')[0]")
            .bind(session_token)
            .await;

        let user: Option<Option<User>> = match response {
            Ok(mut data) => data.take(0).ok(),
            Err(_) => return Outcome::Forward(Status::Unauthorized),
        };

        if let Some(Some(user)) = user {
            Outcome::Success(user)
        } else {
            Outcome::Forward(Status::Unauthorized)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Room {
    name: String,
    // messages: Vec<Message>,
}

struct Message {
    id: u32,
    content: String,
}

#[derive(FromForm, Debug, Serialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(FromForm, Debug, Serialize)]
pub struct RegisterForm {
    username: String,
    email: String,
    password: String,
}

pub enum LoginResult {
    Id(String),
    WrongPassword,
    NewUser,
}

#[post("/login/checkuser", data = "<login_form>")]
pub async fn check_user(login_form: Form<LoginForm>, jar: &CookieJar<'_>) -> String {
    match get_user(&login_form.email, &login_form.password).await {
        LoginResult::NewUser => "NewUser".to_string(),
        LoginResult::WrongPassword => "WrongPassword".to_string(),
        LoginResult::Id(id) => create_session(id, jar).await,
    }
}

#[post("/register/process", data = "<register_form>")]
pub async fn register_user(register_form: Form<RegisterForm>) -> Redirect {
    match db_create_user(register_form.into_inner()).await {
        Ok(_) => {
            Redirect::to("/login?success=Successfully%20registered,%20Please%20log%20in%20now")
        }
        Err(_) => Redirect::to(
            "/register?error=An%20error%20occurred%20while%20registering,%20try%20again%20later",
        ),
    }
}

pub async fn get_user(email: &String, password: &String) -> LoginResult {
    #[derive(Serialize)]
    struct EmailPassword<'a> {
        email: &'a String,
        password: &'a String,
    }

    let db = connect_to_db().await;

    let user: Option<String> = query_one(&db, 
        "type::string((SELECT id FROM user WHERE email= $email AND crypto::bcrypt::compare(password_hash, $password))[0].id)",
        EmailPassword{email,password}).await;

    match user {
        None => {
            if user_exists(email).await {
                return LoginResult::WrongPassword;
            }
            LoginResult::NewUser
        }
        Some(id) => LoginResult::Id(id),
    }
}

async fn user_exists(email: &String) -> bool {
    let db = connect_to_db().await;

    let records: Option<String> = query_one(
        &db,
        "SELECT email FROM user WHERE email= $email",
        ("email", email),
    )
    .await;

    !records.is_some()
}

pub async fn connect_to_db() -> Surreal<ws::Client> {
    let db = Surreal::new::<ws::Ws>("localhost:5000").await.unwrap();

    db.signin(surrealdb::opt::auth::Root {
        username: "root",
        password: env!("DB_PASSWORD"),
    })
    .await
    .unwrap();

    db.use_ns("tobychat").use_db("main").await.unwrap();
    db
}

pub async fn create_session(id: String, jar: &CookieJar<'_>) -> String {
    let db = connect_to_db().await;
    dbg!(&id);

    db.query("UPDATE $id SET session=rand::uuid::v7()")
        .bind(("id", &id))
        .await
        .unwrap();

    let session: Option<Uuid> = query_one(&db, "RETURN $id.session", ("id", &id)).await;
    let session = session.unwrap().to_raw();

    let cookie = Cookie::build(("session", session)).secure(true);

    jar.add_private(cookie);

    "LoggedIn".to_string()
}

pub async fn db_create_user(user_info: RegisterForm) -> Result<(), surrealdb::Error> {
    let db = connect_to_db().await;

    db.query(
        "
    CREATE user SET
    username = $username,
    email = $email,
    password_hash = crypto::bcrypt::generate($password),
    rooms = []
        ",
    )
    .bind(user_info)
    .await?;

    Ok(())
}

pub async fn query_one<B: Serialize, T: for<'a> Deserialize<'a>>(
    db: &Surreal<Client>,
    query: &str,
    bindings: B,
) -> Option<T> {
    let mut response = db.query(query).bind(bindings).await.unwrap();
    let result: Option<Option<T>> = response.take(0).ok();
    result?
}
