use rocket::{fs::NamedFile, get, launch, routes};

#[get("/<file>")]
async fn serve_page(file: &str) -> Option<NamedFile> {
    NamedFile::open(format!("views/{}.html", file)).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![serve_page])
}
