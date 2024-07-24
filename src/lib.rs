use rocket::{
    form::{Form, FromForm},
    get,
    http::{Cookie, CookieJar, Status},
    post,
    request::{FromRequest, Outcome, Request},
    response::{
        stream::{Event, EventStream},
        Redirect,
    },
    serde::Serialize,
    tokio::{
        select,
        sync::broadcast::{error::RecvError, Sender},
    },
    Shutdown, State,
};
use rocket_dyn_templates::{context, Template};
use serde::Deserialize;
use std::str::FromStr;
use surrealdb::{
    engine::remote::ws,
    opt::RecordId,
    sql::{Datetime, Uuid},
    Surreal,
};

#[derive(serde::Deserialize, serde::Serialize, Debug)]
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
        let _ = env!("ROCKET_SECRET_KEY");

        let session_token: Option<Uuid> = match req.cookies().get_private("session") {
            Some(token) => Uuid::from_str(token.value()).ok(),
            None => return Outcome::Forward(Status::Unauthorized),
        };

        let session_token = match session_token {
            Some(token) => token,
            None => return Outcome::Forward(Status::BadRequest),
        };

        let db = connect_to_db().await;

        let mut response = db
            .query(
                "SELECT email,username,created,
                (SELECT META::Id(id) as id, name,created, <-join<-user.username as members FROM ->join->room)
                AS rooms FROM ONLY user 
                WHERE session=$session_token LIMIT 1",
            )
            .bind(("session_token", session_token))
            .await
            .unwrap();

        let user: Result<Option<User>, surrealdb::Error> = response.take(0);

        if let Ok(Some(user)) = user {
            Outcome::Success(user)
        } else {
            Outcome::Forward(Status::Unauthorized)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Room {
    id: String,
    name: String,
    created: Datetime,
    members: Vec<String>,
    // messages: Vec<Message>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Message {
    author: String,
    room: RecordId,
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
    Id(RecordId),
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

#[get("/room/<room_id>")]
pub async fn room(room_id: &str, user: User) -> Option<Template> {
    let room = get_room(room_id, &user).await;

    room.map(|room| Template::render("room", context! {room,user}))
}

#[derive(FromForm)]
pub struct SendMessageForm {
    message: String,
}

#[post("/room/<room_id>", data = "<message_form>")]
pub async fn post_message(
    room_id: &str,
    user: User,
    message_form: Form<SendMessageForm>,
    queue: &State<Sender<Message>>,
) -> Status {
    let binding = Message {
        author: user.email,
        room: RecordId::from_str(&format!("room:{room_id}")).unwrap(),
        content: message_form.message.clone(),
    };
    connect_to_db().await.query("RELATE (SELECT id FROM ONLY user WHERE email=$author LIMIT 1)->send_message->$room SET content=$content;").bind(binding).await.unwrap();

    let result = queue.send(Message {
        author: user.username,
        content: message_form.message.to_string(),
        room: RecordId::from_str(&format!("room:{room_id}")).unwrap(),
    });

    match result {
        Ok(..) => Status::Ok,
        Err(..) => Status::BadRequest,
    }
}

#[get("/room/<room_id>/stream")]
pub async fn room_stream(
    room_id: &str,
    user: User,
    queue: &State<Sender<Message>>,
    mut end: Shutdown,
) -> Option<EventStream![]> {
    let mut rx = queue.subscribe();
    let room_id = room_id.to_string();
    let room_id_copy = room_id.clone();

    let stream = EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

        if msg.room.to_string()==format!("room:{room_id}"){
            dbg!(&msg);
            yield Event::json(&msg);
        }
        }
    };
    if user
        .rooms
        .iter()
        .map(|room| room.id.clone())
        .collect::<Vec<String>>()
        .contains(&room_id_copy)
    {
        Some(stream)
    } else {
        None
    }
}

#[get("/logout")]
pub async fn logout(user: User, jar: &CookieJar<'_>) -> Redirect {
    connect_to_db()
        .await
        .query("UPDATE ONLY user SET session=NONE WHERE email=$email")
        .bind(("email", user.email))
        .await
        .unwrap();
    jar.remove_private("session");
    Redirect::to("/login")
}

async fn get_room(room_id: &str, user: &User) -> Option<Room> {
    #[derive(Serialize)]
    struct RoomIdAndEmail<'a> {
        room_id: &'a str,
        email: String,
    }

    let db = connect_to_db().await;

    let mut response = db
        .query(
            "SELECT Meta::id(id) as id, name, created, <-join<-user.username as members
            FROM ONLY type::thing('room',$room_id)
            WHERE $email in <-join<-user.email",
        )
        .bind(RoomIdAndEmail {
            room_id,
            email: user.email.clone(),
        })
        .await
        .unwrap();

    let room: Option<Option<Room>> = response.take(0).ok();

    if let Some(Some(room)) = room {
        Some(room)
    } else {
        None
    }
}

pub async fn get_user(email: &String, password: &String) -> LoginResult {
    #[derive(Serialize)]
    struct EmailPassword<'a> {
        email: &'a String,
        password: &'a String,
    }

    let db = connect_to_db().await;
    let mut response = db
        .query("SELECT VALUE id FROM ONLY user WHERE email=$email AND crypto::bcrypt::compare(password_hash, $password) LIMIT 1")
        .bind(EmailPassword{email,password}).await.unwrap();

    let user: Option<Option<RecordId>> = response.take(0).ok();

    match user {
        None => {
            if user_exists(email).await {
                return LoginResult::WrongPassword;
            }
            LoginResult::NewUser
        }
        Some(None) => {
            if user_exists(email).await {
                return LoginResult::WrongPassword;
            }
            LoginResult::NewUser
        }
        Some(Some(id)) => LoginResult::Id(id),
    }
}

async fn user_exists(email: &String) -> bool {
    let db = connect_to_db().await;
    let mut response = db
        .query("$email IN SELECT VALUE email FROM user")
        .bind(("email", email))
        .await
        .unwrap();

    let exists: bool = response.take::<Option<bool>>(0).unwrap().unwrap();

    exists
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

pub async fn create_session(id: RecordId, jar: &CookieJar<'_>) -> String {
    let db = connect_to_db().await;

    let mut response = db
        .query("SELECT VALUE session FROM ONLY (UPDATE $id SET session=rand::uuid::v7()) LIMIT 1")
        .bind(("id", id))
        .await
        .unwrap();

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
    password_hash = crypto::bcrypt::generate($password),
    rooms = []
        ",
    )
    .bind(user_info)
    .await?;

    Ok(())
}
