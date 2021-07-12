use rocket::data::{Data, ToByteUnit};
use rocket::http::{self, ContentType};
use rocket::tokio::io::AsyncWriteExt;
use rocket::response::{self, Responder, Response};
use rocket::request::Request;

use multer::{Constraints, Multipart, SizeLimit};

use std::path::{Path, PathBuf};

use super::FILES_DIR;
use super::UPLOAD_SIZE_LIMIT;

pub struct UploadResponse {
    pub status: http::Status,
}

impl UploadResponse {
    fn redirect() -> Self {
        UploadResponse {
            status: http::Status::SeeOther,
        }
    }
    fn bad_request() -> Self {
        UploadResponse {
            status: http::Status::BadRequest,
        }
    }
    fn forbidden() -> Self {
        UploadResponse {
            status: http::Status::Forbidden,
        }
    }
}

impl<'r> Responder<'r, 'r> for UploadResponse {
    fn respond_to(self, request: &'r Request<'_>) -> response::Result<'r> {
        let mut response = Response::build();
        response.status(self.status);
        if self.status == http::Status::SeeOther {
            response.header(http::Header::new("Location", request.uri().path().as_str()));
            response.header(http::Header::new("Cache-Control", "no-cache"));
        }
        response.ok()
    }
}

#[post("/<url_path..>", data = "<files>")]
pub async fn upload_handler(url_path: PathBuf, content_type: &ContentType, files: Data<'_>) -> UploadResponse {
    if content_type.is_form_data() {
        let stream = files.open(UPLOAD_SIZE_LIMIT.mebibytes());
        let content_type = content_type.to_string();
        let (_, boundary) = content_type.split_at(30);
        let constraints = Constraints::new()
            .allowed_fields(vec!["files"])
            .size_limit(SizeLimit::new().whole_stream(UPLOAD_SIZE_LIMIT * 1024 * 1024));
        let multipart = Multipart::with_reader_with_constraints(stream, boundary, constraints);

        fn sanitize_file_name(file_name: Option<&str>) -> Option<String> {
            match file_name {
                Some(file_name) => {
                    let extension = Path::new(file_name).extension()?.to_str()?;
                    let base_name = rocket::fs::FileName::new(file_name).as_str()?;
                    let mut sanitized = base_name.to_owned();
                    sanitized.push('.');
                    sanitized.push_str(extension);
                    Some(sanitized)
                }
                None => None,
            }
        }

        async fn process_uploads(mut multipart: Multipart<'_>, url_path: PathBuf) -> Option<()> {
            while let Some(mut field) = multipart.next_field().await.ok()? {
                let file_name = sanitize_file_name(field.file_name())?;
                let path = Path::new(FILES_DIR).join(&url_path).join(&file_name);
                if path.exists() {
                    return None;
                }                

                let mut file = rocket::tokio::fs::File::create(path).await.ok()?;
                while let Some(chunk) = field.chunk().await.ok()? {
                    file.write_all(&chunk).await.ok()?;
                }
                file.sync_all().await.ok()?;
            }
            Some(())
        }

        match process_uploads(multipart, url_path).await {
            Some(_) => UploadResponse::redirect(),
            None => UploadResponse::bad_request(),
        }
    } else {
        UploadResponse::forbidden()
    }
}
