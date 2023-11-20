mod error;
mod extractors;
pub mod handlers;
mod tools;

use std::{
    env,
    path::{Path, PathBuf},
};

use clap::Parser;
use dashmap::DashMap;
use llm_sdk::LlmSdk;
use tokio::sync::broadcast;

#[derive(Debug, Parser)]
#[clap(name = "ava")]
pub struct Args {
    #[clap(short, long, default_value = "8080")]
    pub port: u16,
    #[clap(short, long, default_value = "./.certs")]
    pub cert_path: String,
}

#[derive(Debug)]
pub struct AppState {
    pub(crate) llm: LlmSdk,
    // each device_id has a channel to send messages to
    pub(crate) signals: DashMap<String, broadcast::Sender<String>>,
    pub(crate) chats: DashMap<String, broadcast::Sender<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            llm: LlmSdk::new(
                "https://api.openai.com/v1",
                env::var("OPENAI_API_KEY").unwrap(),
                3,
            ),
            signals: DashMap::new(),
            chats: DashMap::new(),
        }
    }
}

pub fn audio_path(device_id: &str, name: &str) -> PathBuf {
    Path::new("/tmp/ava-bot/audio")
        .join(device_id)
        .join(format!("{}.mp3", name))
}

pub fn audio_url(device_id: &str, name: &str) -> String {
    format!("/assets/audio/{}/{}.mp3", device_id, name)
}

pub fn image_path(device_id: &str, name: &str) -> PathBuf {
    Path::new("/tmp/ava-bot/image")
        .join(device_id)
        .join(format!("{}.png", name))
}

pub fn image_url(device_id: &str, name: &str) -> String {
    format!("/assets/image/{}/{}.png", device_id, name)
}
