use rocket::{
    response::*,
    form::{Form, FromForm}, post
};
use serde::Deserialize;
use surrealdb::engine::remote::ws;
use surrealdb::Surreal;

struct User {
    id: u32,
    email: String,
    username: String,
    password: String,
    //rooms: Vec<Room>,
}

struct Room {
    id: u32,
    messages: Vec<Message>,
}

struct Message {
    id: u32,
    content: String,
}

#[derive(FromForm)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[post("/login/checkuser", data = "<login_form>")]
pub async fn check_user(login_form: Form<LoginForm>) -> Redirect {
    let id = get_user(&login_form.email,&login_form.password).await;
    match id {
    None => rocket::response::Redirect::to("/register"),
    Some(_id) => rocket::response::Redirect::to("/"),
}
}

pub async fn get_user(email:&String,password:&String)->Option<String>{
    let db = connect_to_db().await;
    let mut response = db
        .query(format!("type::string((SELECT id FROM user WHERE email='{email}' AND password='{password}')[0].id)")).await.unwrap();

    let user:Option<String> = response.take(0).ok()?;

    user
}

pub async fn connect_to_db() -> Surreal<ws::Client> {
    let db = Surreal::new::<ws::Ws>("localhost:5000").await.unwrap();

    db.signin(surrealdb::opt::auth::Root{
        username: "root",
        password: env!("DB_PASSWORD"),
    }).await.unwrap();

    db.use_ns("tobychat").use_db("main").await.unwrap();
    db
}
