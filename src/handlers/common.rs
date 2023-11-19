use askama::Template;
use axum::response::IntoResponse;

#[derive(Debug, Template)]
#[template(path = "index.html.j2")]
struct IndexTemplate {}

pub async fn index_page() -> impl IntoResponse {
    IndexTemplate {}
}
