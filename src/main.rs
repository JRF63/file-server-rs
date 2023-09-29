mod index;
mod statics;
#[cfg(target_os = "windows")]
mod windows;

use actix_web::{web, App, HttpServer};
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let app_state = AppState::new(&args.root);
    let app_state_ref = web::Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .app_data(app_state_ref.clone())
            .service(statics::favicon)
            .service(statics::css)
            .default_service(web::to(index::index))
    })
    .bind(args.addr)?
    .run()
    .await
}
