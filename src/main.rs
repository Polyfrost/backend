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
    let loader_type = match loader.as_str() {
        // Handle fabric versions
        "fabric" => match version.as_str() {
            // "1.16.2" => "prelaunch",
            _ => {
                return HttpResponse::UnprocessableEntity().json(structs::ErrorResponse {
                    error: "INVALID_VERSION".to_string(),
                    message: format!("Version {version} is invalid for fabric loader!"),
                })
            }
        },
        // Handle forge versions
        "forge" => match version.as_str() {
            "1.8.9" | "1.12.2" => "launchwrapper",
            // "1.16.2" => "modlauncher",
            _ => {
                return HttpResponse::UnprocessableEntity().json(structs::ErrorResponse {
                    error: "INVALID_VERSION".to_string(),
                    message: format!("Version {version} is invalid for forge loader!"),
                })
            }
        },
        // Handle invalid versions
        _ => {
            return HttpResponse::UnprocessableEntity().json(structs::ErrorResponse {
                error: "INVALID_LOADER".to_string(),
                message: format!("Loader {loader} is invalid!"),
            })
        }
    };
    // Fetch maven releases data
    let client = &data.http_client;
    let maven_releases_text: Result<String, reqwest::Error> = try {
        client
            .get(format!(
                "{0}/releases/cc/polyfrost/oneconfig-{version}-{loader}/maven-metadata.xml",
                data.internal_maven_url
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
    let (latest_release, latest_release_time) = check_internal_error!(latest_release, "Unable to parse latest release from maven");

    // Fetch maven snapshots data
    let maven_snapshots_text: Result<String, reqwest::Error> = try {
        client
            .get(format!(
                "{0}/releases/cc/polyfrost/oneconfig-{version}-{loader}/maven-metadata.xml",
                data.internal_maven_url
            ))
            .send()
            .await?
            .text()
            .await?
    };
    let maven_snapshots_text = check_internal_error!(maven_snapshots_text);

    let maven_snapshots = XMLDocument::parse(&maven_snapshots_text);
    let maven_snapshots = check_internal_error!(maven_snapshots);

    let latest_snapshot = maven_snapshots.root().get_latest();
    let (latest_snapshot, latest_snapshot_time) = check_internal_error!(latest_snapshot, "Unable to parse latest snapshot from maven");

    // Fetch loader maven data
    let maven_loader_text: Result<String, reqwest::Error> = try {
        client
            .get(format!(
                "{0}/releases/cc/polyfrost/oneconfig-loader-{loader_type}/maven-metadata.xml",
                data.internal_maven_url
            ))
            .send()
            .await?
            .text()
            .await?
    };
    let maven_loader_text = check_internal_error!(maven_loader_text);

    let maven_loader = XMLDocument::parse(&maven_loader_text);
    let maven_loader = check_internal_error!(maven_loader);

    let latest_loader = maven_loader.root().get_latest();
    let (latest_loader, _) = check_internal_error!(latest_loader, "Unable to parse latest loader from maven");

    // Construct the release data
    let release_url = format!("{0}/releases/cc/polyfrost/oneconfig-{version}-{loader}/{latest_release}/oneconfig-{version}-{loader}-{latest_release}-full.jar", data.public_maven_url);
    let release_sha: Result<String, reqwest::Error> = try {
        client
            .get(format!("{release_url}.sha256"))
            .send()
            .await?
            .text()
            .await?
    };
    let releases_info = structs::MavenDataResponse {
        url: release_url,
        sha256: check_internal_error!(
            release_sha
        )
    };

    // Construct the snapshot data
    let snapshot_url = format!("{0}/snapshots/cc/polyfrost/oneconfig-{version}-{loader}/{latest_snapshot}/oneconfig-{version}-{loader}-{latest_snapshot}-full.jar", data.public_maven_url);
    let snapshot_sha: Result<String, reqwest::Error> = try {
        client
            .get(format!("{snapshot_url}.sha256"))
            .send()
            .await?
            .text()
            .await?
    };

    let snapshot_info = if latest_release_time >= latest_snapshot_time {
        releases_info.clone()
    } else {
        structs::MavenDataResponse {
            url: snapshot_url,
            sha256: check_internal_error!(snapshot_sha)
        }
    };

    // Construct the loader data
    let loader_url = format!("{0}/releases/cc/polyfrost/oneconfig-loader-{loader_type}/{latest_loader}/oneconfig-loader-{loader_type}-{latest_loader}.jar", data.public_maven_url);
    let loader_sha: Result<String, reqwest::Error> = try {
        client
            .get(format!("{loader_url}.sha256"))
            .send()
            .await?
            .text()
            .await?
    };

    let loader_info = structs::MavenDataResponse {
        url: loader_url,
        sha256: check_internal_error!(loader_sha)
    };

    // Return all the combined maven info
    HttpResponse::Ok().json(
        structs::OneconfigDataResponse {
            release: releases_info,
            snapshot: snapshot_info,
            loader: loader_info
        }
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port = env::var("PORT").unwrap_or("8080".to_string()).parse::<u16>().expect("Unable to convert PORT variable to number");
    println!("Starting server on port {port}");
    HttpServer::new(|| {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(header::USER_AGENT, HeaderValue::from_static(USER_AGENT));

        App::new()
            .app_data(
                // Add app data accessible in routes
                Data::new(structs::AppState {
                    // Set public maven url to PUBLIC_MAVEN_URL env var, with the default being the normal https url
                    public_maven_url: env::var("PUBLIC_MAVEN_URL")
                        .unwrap_or("https://repo.polyfrost.cc".to_string()),
                    // Set internal maven url to INTERNAL MAVEN_URL env var, with the default being localhost
                    internal_maven_url: env::var("INTERNAL_MAVEN_URL")
                        .unwrap_or("http://localhost:8080".to_string()),
                    // Create a global http client to be used
                    http_client: Client::builder()
                        .default_headers(default_headers)
                        .build()
                        .expect("unable to build http client"),
                }),
            )
            .service(oneconfig)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
