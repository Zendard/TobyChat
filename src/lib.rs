use rocket::{
    form::{Form, FromForm},
    http::{Cookie, CookieJar, Status},
    post,
    request::{FromRequest, Outcome, Request},
    response::Redirect,
    serde::Serialize,
};
use surrealdb::{
    engine::remote::ws,
    sql::{Datetime, Uuid},
    Surreal,
};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct User {
    email: String,
    username: String,
    //rooms: Vec<Room>,
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

        let user: Option<Option<User>> = db
            .query(format!(
                "(SELECT email,username,created FROM user WHERE session='{session_token}')[0]"
            ))
            .await
            .unwrap()
            .take(0)
            .ok();

        if let Some(user) = user {
            if let Some(user) = user {
                Outcome::Success(user)
            } else {
                Outcome::Forward(Status::BadRequest)
            }
        } else {
            Outcome::Forward(Status::InsufficientStorage)
        }
    }
}

struct Room {
    id: u32,
    messages: Vec<Message>,
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
        Ok(_) => Redirect::to("/login?register=sucess"),
        Err(_) => Redirect::to("/register?register=error"),
    }
}

pub async fn get_user(email: &String, password: &String) -> LoginResult {
    let db = connect_to_db().await;
    let mut response = db
        .query(format!("type::string((SELECT id FROM user WHERE email='{email}' AND crypto::bcrypt::compare(password_hash, '{password}'))[0].id)")).await.unwrap();

    let user: Option<Option<String>> = response.take(0).ok();

    match user {
        None => {
            if user_exists(email).await {
                return LoginResult::WrongPassword;
            }
            LoginResult::NewUser
        }
        Some(id) => LoginResult::Id(id.unwrap()),
    }
}

async fn user_exists(email: &String) -> bool {
    let db = connect_to_db().await;
    let mut response = db
        .query(format!("SELECT email FROM user WHERE email='{email}'"))
        .await
        .unwrap();

    let records: Vec<String> = response.take((0, "email")).unwrap();

    !records.is_empty()
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

    db.query(format!("UPDATE {id} SET session=rand::uuid::v7()"))
        .await
        .unwrap();
    let mut response = db.query(format!("RETURN {id}.session")).await.unwrap();

    let session: Option<Uuid> = response.take(0).unwrap();
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
    password_hash = crypto::bcrypt::generate($password)
        ",
    )
    .bind(user_info)
    .await?;

    Ok(())
}
