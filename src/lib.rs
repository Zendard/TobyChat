use rocket::{
    response::*,
    form::{Form, FromForm}, post
};
use surrealdb::engine::remote::ws;
use surrealdb::Surreal;

struct User {
    id: u32,
    email: String,
    username: String,
    password: String,
    rooms: Vec<Room>,
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
    let id = get_user(login_form.email.to_owned(),login_form.password.to_owned()).await;
    match id {
    None => rocket::response::Redirect::to("/register"),
    Some(_id) => rocket::response::Redirect::to("/"),
}
}

pub async fn get_user(email:String,password:String)->Option<String>{
    let db = connect_to_db().await?;
    dbg!(db.query("SELECT * FROM users").await.unwrap());
    let mut id = db
        .query("SELECT id FROM users WHERE email=$email OR password=$password")
        .bind((email,password)).await.unwrap();
    dbg!(&id);

    id.take(0).unwrap()
}

pub async fn connect_to_db() -> Option<Surreal<ws::Client>> {
    let db = Surreal::new::<ws::Ws>("localhost:5000").await.ok()?;

    db.signin(surrealdb::opt::auth::Root{
        username: "root",
        password: env!("DB_PASSWORD"),
    }).await.ok()?;

    db.use_ns("tobychat").use_db("main").await.ok()?;
    Some(db)
}
