use crate::AppState;
use actix_multipart::Multipart;
use actix_web::{http::header::LOCATION, web, HttpRequest, HttpResponse};
use futures_util::TryStreamExt;
use tokio::io::AsyncWriteExt;

pub type UploadResponseType = Result<HttpResponse, actix_web::Error>;

pub async fn upload(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
    web_path: String,
    mut payload: Multipart,
) -> UploadResponseType {
    // Path on the server
    let local_path = data.serve_from.join(&web_path);

    while let Some(mut field) = payload.try_next().await? {
        // A multipart/form-data stream has to contain `content_disposition`
        let content_disposition = field.content_disposition();

        let path = match content_disposition.get_filename() {
            Some(file_name) => local_path.join(file_name),
            None => {
                return Err(actix_web::error::ErrorInternalServerError(
                    "Unable to get file name of upload",
                ));
            }
        };

        let mut f = tokio::fs::File::create(path).await?;
        while let Some(chunk) = field.try_next().await? {
            f.write_all(&chunk).await?;
        }
    }

    let mut response = HttpResponse::SeeOther();
    response.append_header((LOCATION, req.path()));
    Ok(response.finish())
}
