use rocket::fs::{relative, NamedFile};
use std::path::Path;

#[get("/favicon.png")]
pub async fn favicon() -> Option<NamedFile> {
    let path = Path::new(relative!("static/favicon.png"));
    NamedFile::open(path).await.ok()
}