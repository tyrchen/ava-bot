use super::{AssistantEvent, AssistantStep, SpeechResult};
use crate::{
    audio_path, audio_url, error::AppError, extractors::AppContext, handlers::ChatInputEvent,
    AppState,
};
use anyhow::anyhow;
use askama::Template;
use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use llm_sdk::{
    ChatCompletionMessage, ChatCompletionRequest, LlmSdk, SpeechRequest, WhisperRequestBuilder,
    WhisperRequestType,
};
use serde_json::json;
use std::sync::Arc;
use tokio::{fs, sync::broadcast};
use tracing::info;
use uuid::Uuid;

pub async fn assistant_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
    data: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let device_id = &context.device_id;
    let signal_sender = state
        .signals
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for signal sender"))?
        .clone();
    info!("assistant handler");
    let chat_sender = state
        .chats
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for chat sender"))?
        .clone();

    info!("start assist for {}", device_id);

    match process(&signal_sender, &chat_sender, &state.llm, device_id, data).await {
        Ok(_) => Ok(Json(json!({"status": "done"}))),
        Err(e) => {
            signal_sender.send(error(e.to_string()))?;
            Ok(Json(json!({"status": "error"})))
        }
    }
}

async fn process(
    signal_sender: &broadcast::Sender<String>,
    chat_sender: &broadcast::Sender<String>,
    llm: &LlmSdk,
    device_id: &str,
    mut data: Multipart,
) -> anyhow::Result<()> {
    signal_sender.send(in_audio_upload()).unwrap();

    let Some(field) = data.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };

    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    info!("audio data size: {}", data.len());

    signal_sender.send(in_transcription())?;

    let input = transcript(llm, data.to_vec()).await?;

    chat_sender.send(ChatInputEvent::new(&input).into())?;

    signal_sender.send(in_chat_completion())?;

    let output = chat_completion(llm, &input).await?;

    signal_sender.send(in_speech())?;

    let speech_result = speech(llm, device_id, &output).await?;

    signal_sender.send(complete())?;

    chat_sender.send(speech_result.into())?;
    Ok(())
}

async fn transcript(llm: &LlmSdk, data: Vec<u8>) -> anyhow::Result<String> {
    let req = WhisperRequestBuilder::default()
        .file(data)
        .prompt("If audio language is Chinese, please use Simplified Chinese")
        .request_type(WhisperRequestType::Transcription)
        .build()
        .unwrap();
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

async fn speech(llm: &LlmSdk, device_id: &str, text: &str) -> anyhow::Result<SpeechResult> {
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
    Ok(SpeechResult::new(text, audio_url(device_id, &uuid)))
}

fn in_audio_upload() -> String {
    AssistantEvent::Processing(AssistantStep::UploadAudio).into()
}

fn in_transcription() -> String {
    AssistantEvent::Processing(AssistantStep::Transcription).into()
}

fn in_chat_completion() -> String {
    AssistantEvent::Processing(AssistantStep::ChatCompletion).into()
}

fn in_speech() -> String {
    AssistantEvent::Processing(AssistantStep::Speech).into()
}

#[allow(dead_code)]
fn finish_upload_audio() -> String {
    AssistantEvent::Finish(AssistantStep::UploadAudio).into()
}

#[allow(dead_code)]
fn finish_transcription() -> String {
    AssistantEvent::Finish(AssistantStep::Transcription).into()
}

#[allow(dead_code)]
fn finish_chat_completion() -> String {
    AssistantEvent::Finish(AssistantStep::ChatCompletion).into()
}

#[allow(dead_code)]
fn finish_speech() -> String {
    AssistantEvent::Finish(AssistantStep::Speech).into()
}

fn complete() -> String {
    AssistantEvent::Complete.render().unwrap().into()
}

fn error(msg: impl Into<String>) -> String {
    AssistantEvent::Error(msg.into()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_render() {
        let event = error("error");
        assert_eq!(event, r#"\n<p class=\"text-red-500\">Error: error</p>\n"#);
    }
}
