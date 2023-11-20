use askama::Template;
use axum::response::IntoResponse;
use axum_extra::extract::{cookie::Cookie, CookieJar};
use uuid::Uuid;

const COOKIE_NAME: &str = "device_id";

#[derive(Debug, Template)]
#[template(path = "index.html.j2")]
struct IndexTemplate {}

pub async fn index_page(jar: CookieJar) -> impl IntoResponse {
    let jar = match jar.get(COOKIE_NAME) {
        Some(_) => jar,
        None => {
            let device_id = Uuid::new_v4().to_string();
            let cookie = Cookie::build(COOKIE_NAME, device_id)
                .path("/")
                .secure(true)
                .permanent()
                .finish();
            jar.add(cookie)
        }
    };
    (jar, IndexTemplate {})
}
