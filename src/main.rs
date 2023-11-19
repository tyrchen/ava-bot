use anyhow::Result;
use ava_bot::handlers::{chats_handler, index_page};
use axum::{routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::info;

#[derive(Debug, Parser)]
#[clap(name = "ava")]
struct Args {
    #[clap(short, long, default_value = "8080")]
    port: u16,
    #[clap(short, long, default_value = "./.certs")]
    cert_path: String,
}

#[derive(Debug, Default)]
pub(crate) struct AppState {}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let state = Arc::new(AppState::default());
    let app = Router::new()
        .route("/", get(index_page))
        .route("/chats", get(chats_handler))
        .nest_service("/public", ServeDir::new("./public"))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    info!("Listening on {}", addr);

    let cert = std::fs::read(format!("{}/cert.pem", args.cert_path))?;
    let key = std::fs::read(format!("{}/key.pem", args.cert_path))?;
    let config = RustlsConfig::from_pem(cert, key).await?;
    axum_server::bind_rustls(addr.parse()?, config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
