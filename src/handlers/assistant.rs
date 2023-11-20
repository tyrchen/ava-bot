use crate::{audio_path, audio_url, error::AppError, extractors::AppContext, AppState};
use anyhow::anyhow;
use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use llm_sdk::{
    ChatCompletionMessage, ChatCompletionRequest, LlmSdk, SpeechRequest, WhisperRequest,
};
use serde_json::json;
use std::sync::Arc;
use tokio::fs;
use uuid::Uuid;

pub async fn assistant_handler(
    context: AppContext,
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
    let input = transcript(llm, data.to_vec()).await?;
    let output = chat_completion(llm, &input).await?;
    let audio_url = speech(llm, &context.device_id, &output).await?;

    Ok(Json(
        json!({"len": len, "request": input, "response": output, "audio_url": audio_url}),
    ))
}

async fn transcript(llm: &LlmSdk, data: Vec<u8>) -> anyhow::Result<String> {
    let req = WhisperRequest::transcription(data);
    let res = llm.whisper(req).await?;
    Ok(res.text)
}

async fn chat_completion(llm: &LlmSdk, prompt: &str) -> anyhow::Result<String> {
    let req = ChatCompletionRequest::new(vec![
        ChatCompletionMessage::new_system(
            "I'm an assistant who can answer anything for you",
            "Ava",
        ),
        ChatCompletionMessage::new_user(prompt, ""),
    ]);
    let mut res = llm.chat_completion(req).await?;
    let content = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least one choice"))?
        .message
        .content
        .ok_or_else(|| anyhow!("expect content but no content available"))?;
    Ok(content)
}

async fn speech(llm: &LlmSdk, device_id: &str, text: &str) -> anyhow::Result<String> {
    let req = SpeechRequest::new(text);
    let data = llm.speech(req).await?;
    let uuid = Uuid::new_v4().to_string();
    let path = audio_path(device_id, &uuid);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }
    fs::write(&path, data).await?;
    Ok(audio_url(device_id, &uuid))
}
