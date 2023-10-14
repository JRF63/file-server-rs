use crate::AppState;
use actix_web::{get, http::header, web, HttpResponse};

#[get("/static/{file_name}")]
pub async fn serve_static_file(
    data: web::Data<AppState<'_>>,
    file_name: web::Path<String>,
) -> HttpResponse {
    match data.static_files.get(file_name.as_str()) {
        Some((bytes, mime)) => HttpResponse::Ok()
            .insert_header(header::ContentType(mime.clone()))
            .body(*bytes),
        None => crate::error::error_response(&data.hbs, crate::error::HttpError::NotFound),
    }
}
