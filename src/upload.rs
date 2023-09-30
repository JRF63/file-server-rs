use crate::AppState;
use actix_multipart::Multipart;
use actix_web::{
    http::header::LOCATION,
    web::{self, Payload},
    FromRequest, HttpRequest, HttpResponse,
};
use futures_util::TryStreamExt;
use tokio::io::AsyncWriteExt;

pub type UploadResponseType = HttpResponse;

pub async fn upload(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
    payload: Option<Payload>,
    web_path: String,
) -> HttpResponse {
    async fn inner(
        data: &web::Data<AppState<'_>>,
        req: HttpRequest,
        payload: Option<Payload>,
        web_path: String,
    ) -> Result<HttpResponse, actix_web::Error> {
        // Path on the server
        let local_path = data.serve_from.join(&web_path);

        let mut multipart_payload = match payload {
            Some(p) => {
                let mut inner = p.into_inner();
                Multipart::from_request(&req, &mut inner).await?
            }
            None => {
                return Err(actix_web::error::ErrorInternalServerError(
                    "Missing payload on POST",
                ))
            }
        };

        while let Some(mut field) = multipart_payload.try_next().await? {
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

    match inner(&data, req, payload, web_path).await {
        Ok(http_response) => http_response,
        Err(e) => {
            eprintln!("Upload error: {}", e);
            return HttpResponse::InternalServerError().body(crate::error::render_error(
                &data.hbs,
                crate::error::HttpError::InternalServerError,
            ));
        }
    }
}
