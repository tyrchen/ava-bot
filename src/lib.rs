use std::env;

use clap::Parser;
use dashmap::DashMap;
use llm_sdk::LlmSdk;
use tokio::sync::mpsc;

mod error;
pub mod handlers;

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
