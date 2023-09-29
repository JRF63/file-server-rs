use actix_files::NamedFile;
use actix_web::{get, Responder};

#[get("/favicon.png")]
pub async fn favicon() -> impl Responder {
    NamedFile::open_async("./static/favicon.png").await
}

#[get("/css/layout.css")]
pub async fn css() -> impl Responder {
    NamedFile::open_async("./static/layout.css").await
}