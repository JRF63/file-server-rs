#[macro_use]
extern crate rocket;

mod config;
mod upload;

use std::path::{Path, PathBuf};

use rocket::fs::{relative, FileServer, NamedFile};
use rocket::http;
use rocket::Request;
use rocket_dyn_templates::Template;
use serde::Serialize;

const FILES_DIR: &str = r#"C:\Users\Rafael\Downloads"#;
const UPLOAD_SIZE_LIMIT_MIB: u64 = 256;

#[derive(Serialize)]
struct DirContent {
    url: String,
    file_name: String,
    svg_icon: &'static str,
    size: u64,
    time: u64,
}

#[derive(Serialize)]
struct Breadcrumb {
    url: String,
    segment: String,
}

#[derive(Serialize)]
struct TemplateContext {
    breadcrumbs: Vec<Breadcrumb>,
    contents: Vec<DirContent>,
}

async fn get_dir_contents(dir_path: &PathBuf) -> std::io::Result<Vec<DirContent>> {
    use std::os::windows::fs::MetadataExt;

    let mut directories = vec![];
    let mut files = vec![];
    let mut dir_reader = rocket::tokio::fs::read_dir(dir_path).await?;
    while let Some(entry) = dir_reader.next_entry().await? {
        let file_name = entry
            .file_name()
            .into_string()
            .or(Err(std::io::Error::last_os_error()))?;
        let metadata = entry.metadata().await?;
        let mut url = http::RawStr::new(&file_name).percent_encode().to_string();

        let (vec, icon) = if metadata.is_dir() {
            url.push('/');
            (&mut directories, "folder")
        } else {
            (&mut files, "file")
        };

        vec.push(DirContent {
            url,
            file_name,
            svg_icon: icon,
            size: metadata.file_size(),
            time: metadata.last_write_time(),
        })
    }
    directories.append(&mut files);
    Ok(directories)
}

#[get("/<url_path..>", rank = 11)]
async fn page_indexer(url_path: PathBuf) -> Option<Template> {
    let local_path = Path::new(FILES_DIR).join(&url_path);
    let contents = get_dir_contents(&local_path).await.ok()?;

    let breadcrumbs = {
        let mut tmp = Vec::new();
        let mut url = String::new();
        for component in url_path.components().rev() {
            let segment = component.as_os_str().to_str()?.to_owned();
            tmp.push(Breadcrumb {
                url: url.clone(),
                segment,
            });
            url.push_str("../");
        }
        tmp.reverse();
        tmp
    }; 

    let context = TemplateContext {
        breadcrumbs,
        contents,
    };
    Some(Template::render("main", &context))
}

#[get("/favicon.png")]
async fn favicon() -> Option<NamedFile> {
    let path = Path::new(relative!("static/favicon.png"));
    NamedFile::open(path).await.ok()
}

#[catch(default)]
fn default_catcher(status: http::Status, req: &Request) -> String {
    format!("{} ({})", status, req.uri())
}

#[launch]
fn rocket() -> _ {
    let config = config::rocket_config();
    rocket::custom(config)
        .mount("/", routes![favicon, page_indexer, upload::upload_handler])
        .mount("/", FileServer::from(FILES_DIR))
        .register("/", catchers![default_catcher])
        .attach(Template::fairing())
}
