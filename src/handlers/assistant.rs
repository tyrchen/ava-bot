use super::{AssistantEvent, AssistantStep, SpeechResult};
use crate::{
    audio_path, audio_url,
    error::AppError,
    extractors::AppContext,
    handlers::ChatInputEvent,
    image_path, image_url,
    tools::{
        tool_completion_request, AnswerArgs, AssistantTool, DrawImageArgs, DrawImageResult,
        WriteCodeArgs, WriteCodeResult,
    },
    AppState,
};
use anyhow::{anyhow, bail};
use askama::Template;
use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use comrak::{markdown_to_html_with_plugins, plugins::syntect::SyntectAdapter};
use llm_sdk::{
    ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest, CreateImageRequestBuilder,
    ImageResponseFormat, LlmSdk, SpeechRequest, WhisperRequestBuilder, WhisperRequestType,
};
use serde_json::json;
use std::{str::FromStr, sync::Arc};
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

    let choice = chat_completion_with_tools(llm, &input).await?;

    match choice.finish_reason {
        llm_sdk::FinishReason::Stop => {
            let output = choice
                .message
                .content
                .ok_or_else(|| anyhow!("expect content but no content available"))?;

            signal_sender.send(in_speech())?;
            let speech_result = speech(llm, device_id, &output).await?;
            signal_sender.send(complete())?;
            chat_sender.send(speech_result.into())?;
        }
        llm_sdk::FinishReason::ToolCalls => {
            let tool_call = &choice.message.tool_calls[0].function;
            match AssistantTool::from_str(&tool_call.name) {
                Ok(v) if v == AssistantTool::DrawImage => {
                    signal_sender.send(in_draw_image())?;
                    let ret =
                        draw_image(llm, device_id, serde_json::from_str(&tool_call.arguments)?)
                            .await?;
                    signal_sender.send(complete())?;
                    chat_sender.send(ret.into())?;
                }
                Ok(v) if v == AssistantTool::WriteCode => {
                    signal_sender.send(in_write_code())?;
                    let ret = write_code(llm, serde_json::from_str(&tool_call.arguments)?).await?;
                    signal_sender.send(complete())?;
                    chat_sender.send(ret.into())?;
                }

                Ok(v) if v == AssistantTool::Answer => {
                    signal_sender.send(in_chat_completion())?;
                    let output = answer(llm, serde_json::from_str(&tool_call.arguments)?).await?;
                    signal_sender.send(complete())?;

                    signal_sender.send(in_speech())?;
                    let speech_result = speech(llm, device_id, &output).await?;
                    signal_sender.send(complete())?;
                    chat_sender.send(speech_result.into())?;
                }
                _ => {
                    bail!("no proper tool found at the moment")
                }
            }
        }
        _ => {
            bail!("stop reason not supported")
        }
    }

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

async fn chat_completion_with_tools(
    llm: &LlmSdk,
    prompt: &str,
) -> anyhow::Result<ChatCompletionChoice> {
    let req = tool_completion_request(prompt, "");
    let mut res = llm.chat_completion(req).await?;
    let choice = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least one choice"))?;
    Ok(choice)
}

async fn chat_completion(
    llm: &LlmSdk,
    messages: Vec<ChatCompletionMessage>,
) -> anyhow::Result<String> {
    let req = ChatCompletionRequest::new(messages);
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

async fn draw_image(
    llm: &LlmSdk,
    device_id: &str,
    args: DrawImageArgs,
) -> anyhow::Result<DrawImageResult> {
    let req = CreateImageRequestBuilder::default()
        .prompt(args.prompt)
        .response_format(ImageResponseFormat::B64Json)
        .build()
        .unwrap();
    let mut ret = llm.create_image(req).await?;
    let img = ret
        .data
        .pop()
        .ok_or_else(|| anyhow!("expect at least one data"))?;
    let data = STANDARD.decode(img.b64_json.unwrap())?;
    let uuid = Uuid::new_v4().to_string();
    let path = image_path(device_id, &uuid);
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }
    fs::write(&path, data).await?;
    Ok(DrawImageResult::new(
        image_url(device_id, &uuid),
        img.revised_prompt,
    ))
}

async fn write_code(llm: &LlmSdk, args: WriteCodeArgs) -> anyhow::Result<WriteCodeResult> {
    let messages = vec![
      ChatCompletionMessage::new_system("I'm an expert on coding, I'll write code for you in markdown format based on your prompt", "Ava"),
      ChatCompletionMessage::new_user(args.prompt, ""),
    ];
    let md = chat_completion(llm, messages).await?;

    Ok(WriteCodeResult::new(md2html(&md)))
}

async fn answer(llm: &LlmSdk, args: AnswerArgs) -> anyhow::Result<String> {
    let messages = vec![
        ChatCompletionMessage::new_system("I can help answer anything you'd like to chat", "Ava"),
        ChatCompletionMessage::new_user(args.prompt, ""),
    ];
    Ok(chat_completion(llm, messages).await?)
}

fn md2html(md: &str) -> String {
    let adapter = SyntectAdapter::new("Solarized (dark)");
    let options = comrak::Options::default();
    let mut plugins = comrak::Plugins::default();

    plugins.render.codefence_syntax_highlighter = Some(&adapter);
    markdown_to_html_with_plugins(md, &options, &plugins)
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

fn in_draw_image() -> String {
    AssistantEvent::Processing(AssistantStep::DrawImage).into()
}

fn in_write_code() -> String {
    AssistantEvent::Processing(AssistantStep::WriteCode).into()
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
