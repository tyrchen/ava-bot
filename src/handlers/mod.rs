mod assistant;
mod chats;
mod common;

use askama::Template;
pub use assistant::*;
pub use chats::*;
pub use common::*;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "signal.html.j2")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum AssistantEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum AssistantStep {
    UploadAudio,
    Transcription,
    ChatCompletion,
    Speech,
}

impl From<AssistantEvent> for String {
    fn from(event: AssistantEvent) -> Self {
        event.render().unwrap()
    }
}
