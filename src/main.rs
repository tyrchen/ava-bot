use anyhow::Result;
use ava_bot::{
    handlers::{assistant_handler, chats_handler, index_page, signals_handler},
    AppState, Args,
};
use axum::{
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let state = Arc::new(AppState::default());
    let app = Router::new()
        .route("/", get(index_page))
        .route("/chats", get(chats_handler))
        .route("/signals", get(signals_handler))
        .route("/assistant", post(assistant_handler))
        .nest_service("/public", ServeDir::new("./public"))
        .nest_service("/assets", ServeDir::new("/tmp/ava-bot"))
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
