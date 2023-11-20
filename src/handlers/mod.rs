mod assistant;
mod chats;
mod common;

use askama::Template;
pub use assistant::*;
pub use chats::*;
use chrono::Local;
pub use common::*;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::tools::{DrawImageResult, WriteCodeResult};

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/signal.html.j2")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum AssistantEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/chat_input.html.j2")]
struct ChatInputEvent {
    message: String,
    datetime: String,
    avatar: String,
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "events/chat_reply.html.j2")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum ChatReplyEvent {
    Speech(SpeechResult),
    Image(DrawImageResult),
    Markdown(WriteCodeResult),
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/speech.html.j2")]
struct SpeechResult {
    text: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum AssistantStep {
    UploadAudio,
    Transcription,
    ChatCompletion,
    DrawImage,
    WriteCode,
    Speech,
}

impl ChatInputEvent {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            datetime: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            avatar: "https://i.pravatar.cc/128".to_string(),
            name: "User".to_string(),
        }
    }
}

impl SpeechResult {
    fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
        }
    }
}

impl From<AssistantEvent> for String {
    fn from(event: AssistantEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatInputEvent> for String {
    fn from(event: ChatInputEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<SpeechResult> for ChatReplyEvent {
    fn from(result: SpeechResult) -> Self {
        Self::Speech(result)
    }
}

impl From<SpeechResult> for String {
    fn from(result: SpeechResult) -> Self {
        ChatReplyEvent::from(result).render().unwrap()
    }
}

impl From<DrawImageResult> for ChatReplyEvent {
    fn from(result: DrawImageResult) -> Self {
        Self::Image(result)
    }
}

impl From<DrawImageResult> for String {
    fn from(result: DrawImageResult) -> Self {
        ChatReplyEvent::from(result).render().unwrap()
    }
}

impl From<WriteCodeResult> for ChatReplyEvent {
    fn from(result: WriteCodeResult) -> Self {
        Self::Markdown(result)
    }
}

impl From<WriteCodeResult> for String {
    fn from(result: WriteCodeResult) -> Self {
        ChatReplyEvent::from(result).render().unwrap()
    }
}
