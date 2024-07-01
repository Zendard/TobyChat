use rocket::{
    form::{Form, FromForm}, post
};
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

pub enum LoginResult{
    Id(String),
    WrongPassword,
    NewUser
}

#[post("/login/checkuser", data = "<login_form>")]
pub async fn check_user(login_form: Form<LoginForm>) -> String{
    match get_user(&login_form.email,&login_form.password).await{
        LoginResult::Id(id) => id,
        LoginResult::NewUser => "NewUser".to_string(),
        LoginResult::WrongPassword => "WrongPassword".to_string()
    }
}

pub async fn get_user(email:&String,password:&String)-> LoginResult {
    let db = connect_to_db().await;
    let mut response = db
        .query(format!("type::string((SELECT id FROM user WHERE email='{email}' AND password='{password}')[0].id)")).await.unwrap();

    let user:Option<Option<String>> = response.take(0).ok();

    match user{
       None=>{if user_exists(email).await{
            return LoginResult::WrongPassword
        }
        return LoginResult::NewUser
    },
    Some(id)=>return LoginResult::Id(id.unwrap())
    }

}

async fn user_exists(email:&String) -> bool {
    let db= connect_to_db().await;
    let mut response = db.query(format!("SELECT email FROM user WHERE email='{email}'")).await.unwrap();

   let records:Vec<String> = response.take((0,"email")).unwrap();

   !records.is_empty()
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
