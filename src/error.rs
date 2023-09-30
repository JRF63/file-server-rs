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

pub fn render_error(hbs: &Handlebars<'_>, http_error: HttpError) -> String {
    let context = match http_error {
        HttpError::NotFound => ErrorTemplateContext {
            title: "404: Not Found",
            text: "The requested resource could not be found.",
        },
        HttpError::InternalServerError => ErrorTemplateContext {
            title: "500: Internal Server Error",
            text: "An internal server error occured.",
        },
    };
    hbs.render("error", &context)
        .expect("Handlebars failed at rendering")
}
