use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsString,
    path::PathBuf,
};

use crate::consts::*;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionResponseMessage,
        Role,
    },
    Client,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::runtime::{Runtime, Handle};
use toml;

// options
#[derive(Parser, Clone)]
#[clap(
    version = "1.0",
    author = "Tenkai Kariya",
    about = "Interactive chat with GPT"
)]
pub struct Opts {
    #[clap(short = 'n', long = "new", help = "Start a new chat session")]
    pub new: bool,

    #[clap(
        short = 'm',
        long = "model",
        value_name = "MODEL_NAME",
        help = "Specify the model to use (e.g., gpt-4, gpt-3.5-turbo-16k)"
    )]
    pub model: Option<String>,

    #[clap(short = 'b', long = "batch", help = "Respond to stdin and exit")]
    pub batch: bool,

    #[clap(
        short = 'f',
        long = "include-functions",
        help = "Include chat functions"
    )]
    pub include_functions: bool,

    #[clap(
        short = 'l',
        long = "list-sessions",
        help = "List the models the user has access to"
    )]
    pub list_models: bool,

    #[clap(
        short = 'p',
        long = "print-session",
        value_name = "SESSION_ID",
        default_value = "last-session",
        help = "Print a session to stdout, defaulting to the last session"
    )]
    pub print_session: String,

    #[clap(
        short = 's',
        long = "session",
        help = "Continue from a specified session file",
        value_name = "SESSION_ID"
    )]
    pub continue_session: Option<String>,

    #[clap(
        short = 'i',
        long,
        value_name = "PATH",
        help = "Import a file or directory for GPT to process"
    )]
    pub ingest: Option<OsString>,
}

// GPT Connector types
#[derive(Debug, Deserialize, Clone, Default)]
pub struct GPTSettings {
    pub default: Model,
    pub fallback: Model,
    pub load_session: Option<String>,
    pub save_session: Option<String>,
}

impl GPTSettings {
    fn default() -> Self {
        GPTSettings {
            default: GPT4.clone(),
            fallback: GPT3_TURBO_16K.clone(),
            load_session: None,
            save_session: None,
        }
    }

    pub fn load(path: std::path::PathBuf) -> Self {
        match toml::from_str(std::fs::read_to_string(path).unwrap().as_str()) {
            Ok(settings) => settings,
            Err(_) => GPTSettings::default(),
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct ModelConfig {
    pub name: String,
}
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Model {
    pub(crate) name: String,
    pub(crate) endpoint: String,
    pub token_limit: u32,
}

pub struct ModelsList {
    pub default: Model,
    pub fallback: Model,
}
#[derive(Clone)]
pub struct GPTConnector<'session> {
    pub settings: GPTSettings,
    pub include_functions: bool,
    pub client: Client<OpenAIConfig>,
    pub session_data: &'session Session,
    pub model: Model,
}

pub struct GPTResponse {
    pub role: Role,
    pub content: String,
}

// PDF Parser types
pub struct PdfText {
    pub text: BTreeMap<u32, Vec<String>>, // Key is page number
    pub errors: Vec<String>,
}

// Session Manager types

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChatMessage {
    ChatCompletionRequestMessage,
    ChatCompletionResponseMessage,
}

// impl AsMut<async_openai::types::ChatCompletionRequestMessage> for ChatMessage {
//     fn as_mut(&mut self) -> &mut async_openai::types::ChatCompletionRequestMessage {
//         match self {
//             ChatMessage::ChatCompletionRequestMessage => {
//                 &mut self.as_mut()
//             }
//             _ => panic!("Wrong type"),
//         }
//     }
// }
// impl AsMut<async_openai::types::ChatCompletionResponseMessage> for ChatMessage {
//     fn as_mut(&mut self) -> &mut async_openai::types::ChatCompletionResponseMessage {
//         match self {
//             ChatMessage::ChatCompletionResponseMessage => {
//                 &mut self.as_mut()
//             }
//             _ => panic!("Wrong type"),
//         }
//     }
// }
impl From<ChatCompletionRequestMessage> for ChatMessage {
    fn from(message: ChatCompletionRequestMessage) -> Self {
        message.into()
    }
}

impl From<ChatCompletionResponseMessage> for ChatMessage {
    fn from(message: ChatCompletionResponseMessage) -> Self {
        message.into()
    }
}
impl From<ChatMessage> for async_openai::types::ChatCompletionRequestMessage {
    fn from(message: ChatMessage) -> Self {
        match message {
            ChatMessage::ChatCompletionRequestMessage => message.into(),
            _ => panic!("Wrong type"),
        }
    }
}

impl From<ChatMessage> for async_openai::types::ChatCompletionResponseMessage {
    fn from(message: ChatMessage) -> Self {
        match message {
            ChatMessage::ChatCompletionResponseMessage => message.into(),
            _ => panic!("Wrong type"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub model: Model,
    pub messages: Vec<ChatMessage>,
    pub include_functions: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IngestedData {
    session_id: String,
    file_path: String,
    chunk_num: u32,
    content: String,
}
pub struct SessionManager {
    pub include_functions: bool,
    pub cached_request: Option<Vec<ChatCompletionRequestMessage>>,
    pub session_data: Session,
}
pub struct Message {
    pub role: Role,
    pub content: String,
}

// chunkifier types

#[allow(dead_code)]
pub struct UrlData {
    urls: String,
    data: String,
}
#[allow(dead_code)]
pub struct FilePathData {
    file_paths: String,
    data: String,
}
pub struct IngestData {
    pub text: String,
    pub urls: Vec<String>,
    pub file_paths: Vec<PathBuf>,
}
pub struct Chunkifier {}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandProperty {
    #[serde(rename = "type")]
    pub property_type: String,
    pub description: Option<String>,
    #[serde(rename = "enum", default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommandParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub required: Vec<String>,
    pub properties: std::collections::HashMap<String, CommandProperty>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<CommandParameters>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Commands {
    pub commands: Vec<Command>,
}

// a display function for Message
impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format_chat_message(f, self.role.clone(), self.content.clone())
    }
}

fn format_chat_message(
    f: &mut std::fmt::Formatter<'_>,
    role: Role,
    message: String,
) -> std::fmt::Result {
    match role {
        Role::User => write!(f, "You: {}\n\r", message),
        Role::Assistant => write!(f, "GPT: {}\n\r", message),
        _ => Ok(()),
    }
}
