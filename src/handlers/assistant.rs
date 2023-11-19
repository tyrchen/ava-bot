use std::sync::Arc;

use crate::{error::AppError, AppState};
use anyhow::anyhow;
use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use llm_sdk::{ChatCompletionMessage, ChatCompletionRequest, LlmSdk, WhisperRequest};
use serde_json::json;

pub async fn assistant_handler(
    State(state): State<Arc<AppState>>,
    mut data: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let Some(field) = data.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };

    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };
    let len = data.len();

    let llm = &state.llm;
    let text = transcript(llm, data.to_vec()).await?;

    Ok(Json(json!({"len": len, "request": text, "response": ""})))
}

async fn transcript(llm: &LlmSdk, data: Vec<u8>) -> anyhow::Result<String> {
    let req = WhisperRequest::transcription(data);
    let res = llm.whisper(req).await?;
    Ok(res.text)
}

// async fn chat_completion(llm: &LlmSdk, prompt: &str) -> anyhow::Result<String> {
//     let req = ChatCompletionRequest::new(vec![
//         ChatCompletionMessage::new_system(
//             "I'm an assistant who can answer anything for you",
//             "Ava",
//         ),
//         ChatCompletionMessage::new_user(prompt, ""),
//     ]);
//     let res = llm.chat_completion(req).await?;
//     let content = res.choices[0].message.content.unwrap();
//     Ok(res.text)
// }
