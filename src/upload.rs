use multer::Multipart;
use rocket::data::{Data, ToByteUnit};
use rocket::http::{self, ContentType};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::tokio::io::AsyncWriteExt;
use std::path::{Path, PathBuf};

pub struct Reload;

impl<'r> Responder<'r, 'r> for Reload {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'r> {
        let mut response = Response::build();
        response.status(http::Status::SeeOther);
        response.header(http::Header::new("Location", request.uri().path().as_str()));
        response.ok()
    }
}

pub struct HttpStatus(http::Status);

impl<'r> Responder<'r, 'static> for HttpStatus {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'static> {
        self.0.respond_to(request)
    }
}

#[allow(non_upper_case_globals)]
impl HttpStatus {
    const BadRequest: HttpStatus = HttpStatus(http::Status::BadRequest);
    const Forbidden: HttpStatus = HttpStatus(http::Status::Forbidden);
}

impl From<std::io::Error> for HttpStatus {
    fn from(_: std::io::Error) -> Self {
        HttpStatus(http::Status::InternalServerError)
    }
}

impl From<multer::Error> for HttpStatus {
    fn from(_: multer::Error) -> Self {
        HttpStatus(http::Status::BadRequest)
    }
}

#[post("/<url_path..>", data = "<files>")]
pub async fn upload_handler(
    url_path: PathBuf,
    content_type: &ContentType,
    files: Data<'_>,
) -> Result<Reload, HttpStatus> {
    fn sanitize_file_name(file_name: Option<&str>) -> Option<String> {
        file_name.and_then(|file_name| {
            let extension = Path::new(file_name).extension()?.to_str()?;
            let base_name = rocket::fs::FileName::new(file_name).as_str()?;
            let mut sanitized = base_name.to_owned();
            sanitized.push('.');
            sanitized.push_str(extension);
            Some(sanitized)
        })
    }

    if content_type.is_form_data() {
        let stream = files.open(crate::UPLOAD_SIZE_LIMIT_MIB.mebibytes());
        let content_type = content_type.to_string();
        let (_, boundary) = content_type.split_at(30);
        let mut multipart = Multipart::with_reader(stream, boundary);

        while let Some(mut field) = multipart.next_field().await? {
            let file_name = sanitize_file_name(field.file_name()).ok_or(HttpStatus::BadRequest)?;
            let path = Path::new(crate::FILES_DIR).join(&url_path).join(&file_name);

            if path.exists() {
                return Err(HttpStatus::Forbidden);
            } else {
                let mut file = rocket::tokio::fs::File::create(path).await?;
                while let Some(mut chunk) = field.chunk().await? {
                    while file.write_buf(&mut chunk).await? != 0 {}
                }
                file.sync_all().await?;
            }
        }
        Ok(Reload)
    } else {
        Err(HttpStatus::Forbidden)
    }
}
