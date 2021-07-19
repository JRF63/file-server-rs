use rocket::fs::NamedFile;
use rocket::http::{
    self,
    hyper::{header::CACHE_CONTROL, HeaderName},
};
use rocket::request::Request;
use rocket::response::{self, Responder};
use rocket_dyn_templates::Template;
use serde::Serialize;
use std::path::{Path, PathBuf};

const CUSTOM_HEADERS: [(HeaderName, &str); 1] = [(CACHE_CONTROL, "no-store")];

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

async fn dir_contents(dir_path: &PathBuf) -> std::io::Result<Vec<DirContent>> {
    fn stringify_file_size(file_size: u64) -> String {
        macro_rules! ldexp {
            ($fp:expr, $exp:literal) => {
                $fp * f64::from_bits(((0x3ff + $exp) as u64) << 52)
            };
        }

        const U32_MAX: u64 = u32::MAX as u64;
        
        match file_size {
            0..=U32_MAX => {
                const KIB: u32 = 1 << 10;
                const MIB: u32 = 1 << 20;
                const MIB_M_1: u32 = MIB - 1;
                const GIB: u32 = 1 << 30;
                const GIB_M_1: u32 = GIB - 1;
                
                let file_size = file_size as u32;
                match file_size {
                    0 => "".to_owned(),
                    KIB..=MIB_M_1 => format!("{:.2} kiB", ldexp!(file_size as f64, -10)),
                    MIB..=GIB_M_1 => format!("{:.2} MiB", ldexp!(file_size as f64, -20)),
                    GIB..=u32::MAX => format!("{:.2} GiB", ldexp!(file_size as f64, -30)),
                    _ => format!("{} B", file_size),
                }
            }
            file_size => format!("{:.2} GiB", ldexp!(file_size as f64, -30)),
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

pub enum IndexResponder {
    Directory(Template),
    File(NamedFile),
}

pub struct PageIndex(IndexResponder);

impl<'r> Responder<'r, 'static> for PageIndex {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        let result = match self.0 {
            IndexResponder::Directory(template) => template.respond_to(request),
            IndexResponder::File(named_file) => named_file.respond_to(request),
        };
        result.and_then(|mut response| {
            for (header, value) in &CUSTOM_HEADERS {
                response.set_raw_header(header.as_str(), *value);
            }
            Ok(response)
        })
    }
}

#[get("/<path..>", rank = 0)]
pub async fn page_indexer(path: PathBuf) -> std::io::Result<Option<PageIndex>> {
    let local_path = Path::new(crate::FILES_DIR).join(&path);
    match dir_contents(&local_path).await {
        Ok(contents) => Ok(Some(PageIndex(IndexResponder::Directory(
            render_directory(&path, contents),
        )))),
        Err(e) => match e.kind() {
            std::io::ErrorKind::Other => Ok(Some(PageIndex(IndexResponder::File(
                NamedFile::open(&local_path).await?,
            )))),
            std::io::ErrorKind::NotFound => Ok(None),
            _ => Err(e),
        },
    }
}
