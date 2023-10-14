use actix_web::HttpResponse;
use handlebars::Handlebars;
use serde::Serialize;

#[derive(Serialize)]
struct ErrorTemplateContext {
    title: &'static str,
    text: &'static str,
}

pub enum HttpError {
    NotFound,
    InternalServerError,
}

pub fn error_response(hbs: &Handlebars<'_>, http_error: HttpError) -> HttpResponse {
    let (mut builder, context) = match http_error {
        HttpError::NotFound => (
            HttpResponse::NotFound(),
            ErrorTemplateContext {
                title: "404: Not Found",
                text: "The requested resource could not be found.",
            },
        ),
        HttpError::InternalServerError => (
            HttpResponse::InternalServerError(),
            ErrorTemplateContext {
                title: "500: Internal Server Error",
                text: "An internal server error occured.",
            },
        ),
    };

    builder.body(
        hbs.render_template(crate::ERROR_TEMPLATE, &context)
            .expect("Handlebars failed at rendering"),
    )
}
