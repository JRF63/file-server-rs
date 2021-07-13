#[macro_use]
extern crate rocket;

mod upload;

use std::path::{Path, PathBuf};

use rocket::fs::{relative, FileServer, NamedFile};
use rocket::http;
use rocket_dyn_templates::Template;
use serde::Serialize;

const FILES_DIR: &str = r#"C:\Users\Rafael\Downloads"#;
const UPLOAD_SIZE_LIMIT: u64 = 256;

#[derive(Serialize)]
struct DirContent {
    url: String,
    file_name: String,
    svg_icon: String,
}

#[derive(Serialize)]
struct TemplateContext {
    url_path: String,
    contents: Vec<DirContent>,
}

async fn get_dir_contents(dir_path: &PathBuf) -> std::io::Result<Vec<DirContent>> {
    // use std::os::windows::fs::MetadataExt;
    let mut directories = vec![];
    let mut files = vec![];
    let mut dir_reader = rocket::tokio::fs::read_dir(dir_path).await?;
    while let Some(entry) = dir_reader.next_entry().await? {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let metadata = entry.metadata().await?;
        let mut url = http::RawStr::new(&file_name).percent_encode().to_string();
        if metadata.is_dir() {
            url.push('/');
            directories.push(DirContent { url, file_name, svg_icon: "folder".to_owned() });
        } else {
            files.push(DirContent { url, file_name, svg_icon: "file".to_owned() });
        }
    }
    directories.append(&mut files);
    Ok(directories)
}

#[get("/<url_path..>", rank = 11)]
async fn page_indexer(url_path: PathBuf) -> Option<Template> {
    let local_path = Path::new(FILES_DIR).join(&url_path);
    match get_dir_contents(&local_path).await {
        Ok(contents) => {
            let context = TemplateContext {
                url_path: url_path.to_string_lossy().into_owned().replace("\\", "/"),
                contents,
            };
            Some(Template::render("main", &context))
        }
        Err(_) => {
            None
        }
    }
}

#[get("/favicon.png")]
async fn favicon() -> Option<NamedFile> {
    let path = Path::new(relative!("static/favicon.png"));
    NamedFile::open(path).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![favicon, page_indexer, upload::upload_handler])
        .mount("/", FileServer::from(FILES_DIR))
        .attach(Template::fairing())
}
