use rocket::{
    fs::{FileServer, NamedFile},
    get, launch, routes,
};
use sea_orm::Database;

#[get("/<file>")]
async fn serve_page(file: &str) -> Option<NamedFile> {
    NamedFile::open(format!("views/{}.html", file)).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![serve_page, tobychat::check_user])
        .mount("/public", FileServer::from("public"))
}
