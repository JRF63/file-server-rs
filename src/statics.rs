use crate::AppState;
use actix_files::file_extension_to_mime;
use actix_web::{get, http::header, web, HttpResponse};
use aho_corasick::{AhoCorasick, PatternID};

const CARET: PatternID = PatternID::from_u32_unchecked(0);
const CLOUD: PatternID = PatternID::from_u32_unchecked(1);
const FAVICON: PatternID = PatternID::from_u32_unchecked(2);
const FILE: PatternID = PatternID::from_u32_unchecked(3);
const FOLDER: PatternID = PatternID::from_u32_unchecked(4);
const HOME: PatternID = PatternID::from_u32_unchecked(5);
const LAYOUT: PatternID = PatternID::from_u32_unchecked(6);

pub fn build_aho_corasick() -> AhoCorasick {
    let patterns = &[
        "caret", "cloud", "favicon", "file", "folder", "home", "layout",
    ];
    AhoCorasick::new(patterns).unwrap()
}

#[get("/static/{file_name}")]
pub async fn serve_static_file(
    data: web::Data<AppState<'_>>,
    file_name: web::Path<String>,
) -> HttpResponse {
    macro_rules! include_static_file {
        ($file_name:expr, $extension:expr) => {
            (
                include_bytes!(concat!("../static/", $file_name, ".", $extension))
                    as &'static [u8],
                file_extension_to_mime($extension),
            )
        };
    }

    if let Some(mat) = data.ac.find(file_name.as_str()) {
        let (bytes, mime) = match mat.pattern() {
            CARET => include_static_file!("caret", "svg"),
            CLOUD => include_static_file!("cloud", "svg"),
            FAVICON => include_static_file!("favicon", "png"),
            FILE => include_static_file!("file", "svg"),
            FOLDER => include_static_file!("folder", "svg"),
            HOME => include_static_file!("home", "svg"),
            LAYOUT => include_static_file!("layout", "css"),
            _ => unreachable!(),
        };
        return HttpResponse::Ok()
            .insert_header(header::ContentType(mime.clone()))
            .body(bytes);
    }
    crate::error::error_response(&data.hbs, crate::error::HttpError::NotFound)
}
