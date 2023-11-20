mod error;
mod extractors;
pub mod handlers;

use std::{
    env,
    path::{Path, PathBuf},
};

use clap::Parser;
use dashmap::DashMap;
use llm_sdk::LlmSdk;
use tokio::sync::mpsc;

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
    pub(crate) senders: DashMap<String, mpsc::Sender<serde_json::Value>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            llm: LlmSdk::new(
                "https://api.openai.com/v1",
                env::var("OPENAI_API_KEY").unwrap(),
                3,
            ),
            senders: DashMap::new(),
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
