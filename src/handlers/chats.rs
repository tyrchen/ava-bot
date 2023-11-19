use axum::response::{sse::Event, IntoResponse, Sse};
use futures::stream;
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt as _;
use tracing::info;

pub async fn chats_handler() -> impl IntoResponse {
    info!("user connected");

    // A `Stream` that repeats an event every second
    //
    // You can also create streams from tokio channels using the wrappers in
    // https://docs.rs/tokio-stream
    let stream = stream::repeat_with(|| Event::default().data("<li>hello world!</li>"))
        .map(Ok::<_, Infallible>)
        .throttle(Duration::from_secs(180));

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
