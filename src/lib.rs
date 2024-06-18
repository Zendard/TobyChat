use rocket::{
    form::{Form, FromForm},
    post,
};

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
pub async fn check_user(login_form: Form<LoginForm>) -> Option<String> {
    Some("Example".to_string())
}
