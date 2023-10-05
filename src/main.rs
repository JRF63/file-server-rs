mod error;
mod index;
mod upload;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod os_specific;

use actix_files::Files;
use actix_web::{
    http::Method,
    web::{self, Payload},
    App, Either, HttpRequest, HttpServer,
};
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
    hbs: Handlebars<'reg>,
}

impl<'reg> AppState<'reg> {
    fn new(serve_from: &str) -> Self {
        let serve_from = PathBuf::from(serve_from);
        if !serve_from.is_dir() {
            panic!("Root needs to be a directory");
        }

        let mut hbs = Handlebars::new();
        hbs.register_templates_directory(HANDLEBARS_EXT, HANDLEBARS_TEMPLATE_FOLDER)
            .expect("Error registering Handlebars templates");

        Self { serve_from, hbs }
    }
}

pub async fn catch_all(
    data: web::Data<AppState<'_>>,
    req: HttpRequest,
    payload: Option<Payload>,
) -> Either<index::IndexResponseType, upload::UploadResponseType> {
    // Forward slashes causes Windows to assume it's an absolute path to C:\
    let no_starting_slash = req.path().trim_start_matches('/');

    let path = percent_encoding::percent_decode(no_starting_slash.as_bytes())
        .decode_utf8_lossy()
        .to_string();

    match *req.method() {
        Method::GET => Either::Left(index::index(data, path).await),
        Method::POST => Either::Right(upload::upload(data, req, payload, path).await),
        _ => todo!(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let app_state = AppState::new(&args.root);
    let app_state_ref = web::Data::new(app_state);

    let mut ip_addr = args.addr.ip();
    match ip_addr {
        IpAddr::V4(addr) => {
            if addr.is_unspecified() {
                ip_addr = os_specific::default_ip_address(true)?;
            }
        }
        IpAddr::V6(addr) => {
            if addr.is_unspecified() {
                ip_addr = os_specific::default_ip_address(false)?;
            }
        }
    }
    println!("Serving");
    println!("    Directory: {}", args.root);
    println!("    IP address: http://{}:{}", ip_addr, args.addr.port());

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
