use rocket::{
    catch, catchers,
    form::Form,
    fs::{FileServer, NamedFile},
    get, launch, post,
    response::Redirect,
    routes,
};
use rocket_dyn_templates::Template;
use tobychat::User;

#[get("/<file>")]
async fn serve_page(file: &str) -> Option<NamedFile> {
    NamedFile::open(format!("templates/{file}.html")).await.ok()
}

#[post("/register", data = "<values>")]
async fn register_page(values: Form<tobychat::LoginForm>) -> Template {
    let values = values.into_inner();
    Template::render("register", values)
}

#[get("/")]
async fn index(user: User) -> Template {
    Template::render("index", user)
}

#[catch(401)]
async fn redirect_to_login() -> Redirect {
    Redirect::to("/login?error=Please%20log%20in")
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                serve_page,
                tobychat::check_user,
                register_page,
                tobychat::register_user,
                index,
                tobychat::room,
                tobychat::room_socket
            ],
        )
        .mount("/public", FileServer::from("public"))
        .register("/", catchers![redirect_to_login])
        .attach(Template::fairing())
}
