use rocket::data::{Data, ToByteUnit};
use rocket::http::{self, ContentType};
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::tokio::io::AsyncWriteExt;

use multer::{Constraints, Multipart, SizeLimit};

use std::path::Path;

use super::DOWNLOAD_DIR;
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

impl<'r, 'o: 'r> Responder<'r, 'o> for UploadResponse {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let mut response = Response::build();
        response.status(self.status);
        if self.status == http::Status::SeeOther {
            response.header(http::Header::new("location", "/"));
        }
        response.ok()
    }
}

fn sanitize_file_name(file_name: &str) -> Option<String> {
    let extension = Path::new(file_name).extension()?.to_str()?;
    let base_name = rocket::fs::FileName::new(file_name).as_str()?;
    let mut sanitized = base_name.to_owned();
    sanitized.push('.');
    sanitized.push_str(extension);
    Some(sanitized)
}

#[post("/", data = "<files>")]
pub async fn upload_handler(content_type: &ContentType, files: Data<'_>) -> UploadResponse {
    if content_type.is_form_data() {
        let stream = files.open(UPLOAD_SIZE_LIMIT.mebibytes());
        let content_type = content_type.to_string();
        let (_, boundary) = content_type.split_at(30);
        let constraints = Constraints::new()
            .allowed_fields(vec!["files"])
            .size_limit(SizeLimit::new().whole_stream(UPLOAD_SIZE_LIMIT * 1024 * 1024));
        let multipart = Multipart::with_reader_with_constraints(stream, boundary, constraints);

        async fn process_uploads(mut multipart: Multipart<'_>) -> multer::Result<()> {
            while let Some(mut field) = multipart.next_field().await? {
                let file_name = match field.file_name() {
                    Some(s) => sanitize_file_name(s).ok_or(multer::Error::IncompleteHeaders),
                    None => Err(multer::Error::IncompleteHeaders),
                }?;

                let path = Path::new(DOWNLOAD_DIR).join(&file_name);
                let mut file = rocket::tokio::fs::File::create(path).await.or(Err(multer::Error::IncompleteStream))?;
                while let Some(chunk) = field.chunk().await? {
                    file.write(&chunk).await.or(Err(multer::Error::IncompleteStream))?;
                    // println!("Chunk: {:?}", chunk);
                }
                
                // let bytes = field.bytes().await?;
                // println!("Filename: {}", file_name);
                // println!("Size: {} bytes", bytes.len());
            }
            Ok(())
        }

        async fn process_uploads2(mut multipart: Multipart<'_>) -> Option<()> {
            while let Some(mut field) = multipart.next_field().await.ok()? {
                let file_name = match field.file_name() {
                    Some(s) => sanitize_file_name(s),
                    None => None,
                }?;

                let path = Path::new(DOWNLOAD_DIR).join(&file_name);
                let mut file = rocket::tokio::fs::File::create(path).await.ok()?;
                while let Some(chunk) = field.chunk().await.ok()? {
                    file.write(&chunk).await.ok()?;
                }
            }
            Some(())
        }

        match process_uploads(multipart).await {
            Ok(_) => UploadResponse::redirect(),
            Err(_) => UploadResponse::bad_request(),
        }
    } else {
        UploadResponse::forbidden()
    }
}
