use crate::AppState;
use actix_files::NamedFile;
use actix_web::{
    http::header::http_percent_encode, web, Either, HttpRequest, HttpResponse, Responder,
};
use serde::Serialize;
use std::{
    fmt,
    path::{Path, PathBuf},
};

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

pub async fn index(data: web::Data<AppState<'_>>, req: HttpRequest) -> impl Responder {
    // Forward slashes causes Windows to assume it's an absolute path to C:\
    let no_starting_slash = req.path().trim_start_matches('/');

    let path = percent_encoding::percent_decode(no_starting_slash.as_bytes())
        .decode_utf8_lossy()
        .to_string();

    handle_listing_files(data, path).await
}

async fn handle_listing_files(
    data: web::Data<AppState<'_>>,
    web_path: String,
) -> Either<HttpResponse, std::io::Result<NamedFile>> {
    // Path on the server
    let local_path = data.serve_from.join(&web_path);

    match dir_contents(&local_path).await {
        Ok(contents) => {
            let breadcrumbs = {
                let mut tmp = Vec::new();
                let mut url = String::new();
                for component in Path::new(&web_path).components().rev() {
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
            let body = data
                .handlebars
                .render("main", &context)
                .expect("Handlebars failed at rendering");
            Either::Left(HttpResponse::Ok().body(body))
        }
        Err(_) => Either::Right(NamedFile::open_async(local_path).await),
    }
}

async fn dir_contents(dir_path: &PathBuf) -> std::io::Result<Vec<DirContent>> {
    // Helper struct for percent encoding a string
    struct PercentEncodedStr<'a>(&'a str);

    impl<'a> fmt::Display for PercentEncodedStr<'a> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            http_percent_encode(f, self.0.as_bytes())
        }
    }

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
    let mut dir_reader = tokio::fs::read_dir(dir_path).await?;
    while let Some(entry) = dir_reader.next_entry().await? {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let mut url = format!("{}", PercentEncodedStr(&file_name));

        let metadata = entry.metadata().await?;

        #[cfg(target_os = "windows")]
        let (file_size, modified) = crate::windows::get_metadata(&metadata);

        #[cfg(target_os = "windows")]
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