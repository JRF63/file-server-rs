use rocket::fs::NamedFile;
use rocket::http;
use rocket::http::hyper::header::CACHE_CONTROL;
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct DirContent {
    url: String,
    file_name: String,
    svg_icon: &'static str,
    date: String,
    size: String,
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

pub enum ResponseWrapper {
    Directory(Template),
    File(NamedFile),
}

pub struct PageIndex(ResponseWrapper);

impl<'r> Responder<'r, 'static> for PageIndex {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let result = match self.0 {
            ResponseWrapper::Directory(template) => template.respond_to(request),
            ResponseWrapper::File(named_file) => named_file.respond_to(request),
        };
        result.and_then(|mut response| {
            response.set_raw_header(CACHE_CONTROL.as_str(), "no-store");
            Ok(response)
        })
    }
}

async fn dir_contents(dir_path: &PathBuf) -> std::io::Result<Vec<DirContent>> {
    fn stringify_file_size(file_size: u64) -> String {
        if file_size == 0 {
            return "".to_owned();
        }

        let (converted, prefix) = match file_size {
            1024..=1048575 => (file_size as f64 / 1024.0, "kiB"),
            1048576..=1073741823 => (file_size as f64 / 1048576.0, "MiB"),
            1073741824..=1099511627776 => (file_size as f64 / 1073741824.0, "GiB"),
            _ => (file_size as f64, "B"),
        };

        if file_size < 1024 {
            format!("{} {}", file_size, prefix)
        } else {
            format!("{:.2} {}", converted, prefix)
        }
    }

    let mut directories = vec![];
    let mut files = vec![];
    let mut dir_reader = rocket::tokio::fs::read_dir(dir_path).await?;
    while let Some(entry) = dir_reader.next_entry().await? {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let mut url = http::RawStr::new(&file_name).percent_encode().to_string();

        let metadata = entry.metadata().await?;
        let (file_size, modified) = crate::windows::get_metadata(&metadata);
        let date = crate::windows::get_date(modified)?;

        let (vec, svg_icon) = if metadata.is_dir() {
            url.push('/');
            (&mut directories, "folder")
        } else {
            (&mut files, "file")
        };

        vec.push(DirContent {
            url,
            file_name,
            svg_icon,
            date,
            size: stringify_file_size(file_size),
        })
    }
    directories.append(&mut files);
    Ok(directories)
}

fn render_directory(path: &PathBuf, contents: Vec<DirContent>) -> Template {
    let breadcrumbs = {
        let mut tmp = Vec::new();
        let mut url = String::new();
        for component in path.components().rev() {
            let segment = component.as_os_str().to_string_lossy().into_owned();
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
    Template::render("main", &context)
}

#[get("/<path..>", rank = 0)]
pub async fn page_indexer(path: PathBuf) -> std::io::Result<PageIndex> {
    let local_path = Path::new(crate::FILES_DIR).join(&path);
    let metadata = rocket::tokio::fs::metadata(&local_path).await?;
    let response = if metadata.is_dir() {
        let contents = dir_contents(&local_path).await?;
        ResponseWrapper::Directory(render_directory(&path, contents))
    } else if metadata.is_file() {
        ResponseWrapper::File(NamedFile::open(&local_path).await?)
    } else {
        return Err(std::io::Error::last_os_error());
    };
    Ok(PageIndex(response))
}
