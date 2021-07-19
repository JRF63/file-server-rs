#[macro_use]
extern crate rocket;

mod config;
mod indexer;
mod statics;
mod upload;
mod windows;

use rocket::http;
use rocket::shield::{ExpectCt, NoSniff, Prefetch, Referrer, Shield, XssFilter};
use rocket::Request;
use rocket_dyn_templates::Template;

const FILES_DIR: &str = r#"C:\Users\Rafael\Downloads"#;
const UPLOAD_SIZE_LIMIT_MIB: u64 = 256;

#[catch(default)]
fn default_catcher(status: http::Status, req: &Request) -> String {
    format!("{} ({})", status, req.uri())
}

#[launch]
fn rocket() -> _ {
    let shield = Shield::default()
        .enable(Referrer::NoReferrer)
        .enable(XssFilter::default())
        .enable(NoSniff::Enable)
        .enable(ExpectCt::default())
        .enable(Prefetch::Off);

    let config = config::rocket_config();
    rocket::custom(config)
        .mount(
            "/",
            routes![
                statics::favicon,
                indexer::page_indexer,
                upload::upload_handler
            ],
        )
        .register("/", catchers![default_catcher])
        .attach(Template::fairing())
        .attach(shield)
}
