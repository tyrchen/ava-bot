use askama::Template;
use llm_sdk::{ChatCompletionMessage, ChatCompletionRequest, Tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub(crate) enum AssistantTool {
    /// Draw a picture based on user's input
    DrawImage,
    /// Write code based on user's input
    WriteCode,
    /// Just reply based on user's input
    Answer,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct DrawImageArgs {
    /// The revised prompt for creating the image
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/image.html.j2")]
pub(crate) struct DrawImageResult {
    /// image url
    pub(crate) url: String,
    /// revised prompt
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Template)]
#[template(path = "blocks/markdown.html.j2")]
pub(crate) struct WriteCodeResult {
    /// revised prompt
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct WriteCodeArgs {
    /// The revised prompt for writing the code
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct AnswerArgs {
    /// question or prompt from user
    pub(crate) prompt: String,
}

pub(crate) fn tool_completion_request(
    input: impl Into<String>,
    name: &str,
) -> ChatCompletionRequest {
    let messages = vec![
      ChatCompletionMessage::new_system("I can help to identify which tool to use, if no proper tool could be used, I'll directly reply the message with pure text", "Ava"),
      ChatCompletionMessage::new_user(input.into(), name)
      ];
    ChatCompletionRequest::new_with_tools(messages, all_tools())
}

// TODO: llm-sdk shall provide fuctionality to generate this code
fn all_tools() -> Vec<Tool> {
    vec![
        Tool::new_function::<DrawImageArgs>("draw_image", "Draw an image based on the prompt."),
        Tool::new_function::<WriteCodeArgs>("write_code", "Write code based on the prompt."),
        Tool::new_function::<AnswerArgs>("answer", "Just reply based on the prompt."),
    ]
}

impl DrawImageResult {
    pub(crate) fn new(url: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            prompt: prompt.into(),
        }
    }
}

impl WriteCodeResult {
    pub(crate) fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}
