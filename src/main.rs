use rocket::{
    fs::{FileServer, NamedFile},
    get, launch, routes, post, form::Form
};
use rocket_dyn_templates::Template;

#[get("/<file>")]
async fn serve_page(file: &str) -> Option<NamedFile> {
    NamedFile::open(format!("templates/{file}.html")).await.ok()
}

#[post("/register", data = "<values>")]
async fn register_page(values: Form<tobychat::LoginForm>) -> Template {
    let values = values.into_inner();
    dbg!(&values);
    Template::render("register", values)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![serve_page, tobychat::check_user, register_page])
        .mount("/public", FileServer::from("public"))
        .attach(Template::fairing())
}
