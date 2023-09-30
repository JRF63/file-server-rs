mod index;
mod upload;
#[cfg(target_os = "windows")]
mod windows;

use actix_files::Files;
use actix_multipart::Multipart;
use actix_web::{http::Method, web, App, Either, HttpRequest, HttpServer};
use clap::Parser;
use handlebars::Handlebars;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

const HANDLEBARS_EXT: &str = ".html.hbs";
const HANDLEBARS_TEMPLATE_FOLDER: &str = "./templates";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Root directory of the files to serve
    #[arg(short, long)]
    root: String,

    /// Desired IP address of the server
    #[arg(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))]
    addr: SocketAddr,
}

pub struct AppState<'reg> {
    serve_from: PathBuf,
    handlebars: Handlebars<'reg>,
}

impl<'reg> AppState<'reg> {
    fn new(serve_from: &str) -> Self {
        let serve_from = PathBuf::from(serve_from);
        if !serve_from.is_dir() {
            panic!("Root needs to be a directory");
        }

        let mut handlebars = Handlebars::new();
        handlebars
            .register_templates_directory(HANDLEBARS_EXT, HANDLEBARS_TEMPLATE_FOLDER)
            .expect("Error registering Handlebars templates");

        Self {
            serve_from,
            handlebars,
        }
    }
}

pub async fn catch_all(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
    payload: Option<Multipart>,
) -> Either<index::IndexResponseType, upload::UploadResponseType> {
    // Forward slashes causes Windows to assume it's an absolute path to C:\
    let no_starting_slash = req.path().trim_start_matches('/');

    let path = percent_encoding::percent_decode(no_starting_slash.as_bytes())
        .decode_utf8_lossy()
        .to_string();

    match *req.method() {
        Method::GET => Either::Left(index::index(data, path).await),
        Method::POST => Either::Right(upload::upload(data, req, path, payload.expect("POST has no payload")).await),
        _ => todo!(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let app_state = AppState::new(&args.root);
    let app_state_ref = web::Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state_ref.clone())
            .service(Files::new("/static", "./static"))
            .default_service(web::to(catch_all))
    })
    .bind(args.addr)?
    .run()
    .await
}
