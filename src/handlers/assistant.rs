use axum::{extract::Multipart, response::IntoResponse};

pub async fn assistant_handler(mut data: Multipart) -> impl IntoResponse {
    while let Some(field) = data.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!("Length of `{}` is {} bytes", name, data.len());
    }
    "hello"
}
