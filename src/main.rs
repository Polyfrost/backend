#![feature(try_blocks)]

mod parsing;
mod structs;

#[macro_use]
mod utils;

use actix_web::{get, web::Data, web::Path, App, HttpResponse, HttpServer, Responder};
use const_format::formatcp;
use reqwest::{
    header::HeaderMap,
    header::{self, HeaderValue},
    Client,
};
use roxmltree::Document as XMLDocument;
use std::env;
use parsing::MavenParser;

const USER_AGENT: &str = formatcp!("PolyfrostAPI/{0}", env!("CARGO_PKG_VERSION"));

#[get("/oneconfig/{version}-{loader}")]
async fn oneconfig(data: Data<structs::AppState>, path: Path<(String, String)>) -> impl Responder {
    // Expand params
    let (version, loader) = path.into_inner();
    // Get the type of loader based on version and modloader
    match loader.as_str() {
        // Handle fabric versions
        "fabric" => match version.as_str() {
            "1.16.2" => "prelaunch",
            _ => {
                return HttpResponse::BadRequest().json(structs::ErrorResponse {
                    error: "INVALID_VERSION".to_string(),
                    message: format!("Version {version} is invalid for fabric loader!"),
                })
            }
        },
        // Handle forge versions
        "forge" => match version.as_str() {
            "1.8.9" | "1.12.2" => "launchwrapper",
            "1.16.2" => "modlauncher",
            _ => {
                return HttpResponse::BadRequest().json(structs::ErrorResponse {
                    error: "INVALID_VERSION".to_string(),
                    message: format!("Version {version} is invalid for forge loader!"),
                })
            }
        },
        // Handle invalid versions
        _ => {
            return HttpResponse::BadRequest().json(structs::ErrorResponse {
                error: "INVALID_LOADER".to_string(),
                message: format!("Loader {loader} is invalid!"),
            })
        }
    };
    // Fetch maven data
    let client = &data.http_client;
    let maven_releases_text: Result<String, reqwest::Error> = try {
        client
            .get(format!(
                "{0}/releases/cc/polyfrost/oneconfig-{version}-{loader}/maven-metadata.xml",
                data.maven_url
            ))
            .send()
            .await?
            .text()
            .await?
    };
    let maven_releases_text = check_internal_error!(maven_releases_text);

    let maven_releases = XMLDocument::parse(&maven_releases_text);
    let maven_releases = check_internal_error!(maven_releases);

    let latest_release = maven_releases.root().get_latest();
    let latest_release = check_internal_error!(latest_release, "Unable to parse latest release from maven");

    println!("{}", latest_release);
    // Fetch the urls
    HttpResponse::Ok().body(format!("Version: {version}\nLoader: {loader}"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(header::USER_AGENT, HeaderValue::from_static(USER_AGENT));

        App::new()
            .app_data(
                // Add app data accessible in routes
                Data::new(structs::AppState {
                    // Set maven url to MAVEN_URL env var, with the default being the normal https url
                    maven_url: env::var("MAVEN_URL")
                        .unwrap_or("https://repo.polyfrost.cc".to_string()),
                    // Create a global http client to be used
                    http_client: Client::builder()
                        .default_headers(default_headers)
                        .build()
                        .expect("unable to build http client"),
                }),
            )
            .service(oneconfig)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
