use std::{
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

use async_openai::{config::OpenAIConfig, types::ChatCompletionRequestSystemMessage};
use serde_derive::{Deserialize, Serialize};

use super::{consts::*, functions::CallableFunction, types::Model};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SessionConfig {
  pub prompt: String,
  pub session_id: String,
  pub session_dir: PathBuf,
  pub available_functions: Vec<CallableFunction>,
  pub list_file_paths: Vec<PathBuf>,
  pub model: Model,
  pub name: String,
  pub include_functions: bool,
  pub stream_response: bool,
  pub function_result_max_tokens: usize,
  pub response_max_tokens: usize,
}

impl Default for SessionConfig {
  fn default() -> Self {
    SessionConfig {
      prompt: String::new(),
      session_id: Self::generate_session_id(),
      session_dir: PathBuf::new(),
      available_functions: vec![],
      list_file_paths: vec![],
      model: GPT4_TURBO.clone(),
      name: "Sazid Test".to_string(),
      function_result_max_tokens: 8192,
      response_max_tokens: 4095,
      include_functions: true,
      stream_response: true,
    }
  }
}
impl SessionConfig {
  pub fn prompt_message(&self) -> ChatCompletionRequestSystemMessage {
    ChatCompletionRequestSystemMessage { content: Some(self.prompt.clone()), ..Default::default() }
  }

  pub fn generate_session_id() -> String {
    // Get the current time since UNIX_EPOCH in seconds.
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards").as_secs();

    // Introduce a delay of 1 second to ensure unique session IDs even if called rapidly.
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Convert the duration to a String and return.
    since_the_epoch.to_string()
  }
}
