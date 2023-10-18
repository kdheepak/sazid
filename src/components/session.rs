use async_openai::error::OpenAIError;
use async_openai::types::{
  ChatChoice, ChatCompletionRequestMessage, ChatCompletionResponseMessage, ChatCompletionResponseStreamMessage,
  CreateChatCompletionRequest, CreateChatCompletionResponse, CreateChatCompletionStreamResponse,
  CreateEmbeddingRequestArgs, CreateEmbeddingResponse, Role,
};
use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use futures::StreamExt;
use ratatui::layout::Rect;
use ratatui::{prelude::*, symbols::scrollbar, widgets::block::*, widgets::*};
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io};
use tokio::sync::mpsc::UnboundedSender;

use async_openai::{config::OpenAIConfig, Client};
use async_recursion::async_recursion;
use backoff::exponential::ExponentialBackoffBuilder;

use tokio::runtime::Runtime;

use super::{Component, Frame};
use crate::app::{consts::*, errors::*, tools::chunkifier::*, types::ChatMessage, types::*};
use crate::trace_dbg;
use crate::{
  action::Action,
  config::{Config, KeyBindings},
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::app::gpt_interface::handle_chat_response_function_call;
use crate::app::gpt_interface::{create_chat_completion_function_args, define_commands};
use crate::app::tools::utils::ensure_directory_exists;
use crate::components::home::Mode;

use std::fs::File;
use std::io::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]

pub struct SessionConfig {
  pub session_id: String,
  pub model: Model,
  pub include_functions: bool,
  pub stream_response: bool,
}

impl Default for SessionConfig {
  fn default() -> Self {
    SessionConfig {
      session_id: Self::generate_session_id(),
      model: GPT4.clone(),
      include_functions: false,
      stream_response: true,
    }
  }
}
impl SessionConfig {
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
#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct Session {
  pub transactions: Vec<ChatTransaction>,
  pub config: SessionConfig,
  #[serde(skip)]
  pub action_tx: Option<UnboundedSender<Action>>,
  #[serde(skip)]
  pub mode: Mode,
  #[serde(skip)]
  pub last_events: Vec<KeyEvent>,
  #[serde(skip)]
  pub vertical_scroll_state: ScrollbarState,
  #[serde(skip)]
  pub horizontal_scroll_state: ScrollbarState,
  #[serde(skip)]
  pub vertical_scroll: u16,
  #[serde(skip)]
  pub horizontal_scroll: u16,
}

impl Component for Session {
  fn init(&mut self, area: Rect) -> Result<()> {
    Ok(())
  }
  fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
    trace_dbg!("register_session_action_handler");
    self.action_tx = Some(tx);
    Ok(())
  }
  fn register_config_handler(&mut self, config: Config) -> Result<()> {
    self.config = config.session_config;
    Ok(())
  }
  fn update(&mut self, action: Action) -> Result<Option<Action>> {
    match action {
      Action::SubmitInput(s) => self.request_response(s),
      Action::ProcessResponse(response) => self.process_response_handler(*response),
      _ => (),
    }
    Ok(None)
  }

  fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
    self.last_events.push(key);
    match self.mode {
      Mode::Normal => match key.code {
        KeyCode::Char('j') => {
          self.vertical_scroll = self.vertical_scroll.saturating_add(1);
          self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
          Ok(Some(Action::Update))
        },
        KeyCode::Char('k') => {
          self.vertical_scroll = self.vertical_scroll.saturating_sub(1);
          self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
          Ok(Some(Action::Update))
        },
        _ => Ok(None),
      },
      _ => Ok(None),
    }
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
    let rects = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Percentage(100), Constraint::Min(3)].as_ref())
      .split(area);
    let shorter = Layout::default()
      .direction(Direction::Horizontal)
      .constraints(vec![Constraint::Length(1), Constraint::Min(10), Constraint::Length(1)])
      .split(rects[0]);

    // a function that will return a vec with an aribitrary numbr of the same item

    let textbox = Layout::default()
      .direction(Direction::Vertical)
      .constraints(vec![Constraint::Min(2), Constraint::Length(3)])
      .split(shorter[1]);

    let get_style_from_role = |role| match role {
      Some(Role::User) => Style::default().fg(Color::Yellow),
      Some(Role::Assistant) => Style::default().fg(Color::Green),
      Some(Role::System) => Style::default().fg(Color::Blue),
      Some(Role::Function) => Style::default().fg(Color::Red),
      None => Style::default(),
    };

    let title = "Chat";

    let block = Block::default()
      .borders(Borders::ALL)
      .gray()
      .title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD)));
    let mut text = Vec::new();
    for (index, transaction) in self.transactions.iter().enumerate() {
      let mut style = Style::default().fg(Color::White);
      let mut content = String::new() + "test";
      let messages = <Vec<RenderedChatMessage>>::from(transaction.clone());
      for message in messages.iter() {
        style = get_style_from_role(message.role.clone());
        content = content + message.content.clone().as_str();
      }
      text.push(Line::styled(content, style));
    }
    let block = Block::default()
      .borders(Borders::TOP)
      .gray()
      .title(Title::from("left").alignment(Alignment::Left))
      .title(Title::from("right").alignment(Alignment::Right));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, textbox[0]);

    // f.render_stateful_widget(
    //   Scrollbar::default()
    //     .orientation(ScrollbarOrientation::VerticalRight)
    //     .begin_symbol(Some("↑"))
    //     .end_symbol(Some("↓")),
    //   textbox[1],
    //   &mut self.vertical_scroll_state,
    // );
    Ok(())
  }
}

impl Session {
  pub fn new() -> Session {
    Self::default()
  }

  pub fn request_response(&mut self, input: String) {
    let tx = self.action_tx.clone().unwrap();
    let request_messages = construct_chat_completion_request_message(&input, &self.config.model).unwrap();
    let request = construct_request(request_messages, &self.config);
    let stream_response = self.config.stream_response;
    self.transactions.push(ChatTransaction::Request(request.clone()));
    tokio::spawn(async move {
      tx.send(Action::EnterProcessing).unwrap();
      let client = create_openai_client();
      match stream_response {
        true => {
          let mut stream = client.chat().create_stream(request).await.unwrap();
          let mut file = File::create("saved_response.txt").unwrap();
          while let Some(response_result) = stream.next().await {
            match response_result {
              Ok(response) => {
                let _ = file.write_all(serde_json::to_string(&response).unwrap().as_bytes());
                tx.send(Action::ProcessResponse(Box::new(ChatTransaction::StreamResponse(response)))).unwrap()
              },
              Err(e) => {
                trace_dbg!("Error: {}", e);
                tx.send(Action::Error(format!("Error: {}", e))).unwrap()
              },
            }
          }
        },
        false => match client.chat().create(request).await {
          Ok(response) => tx.send(Action::ProcessResponse(Box::new(ChatTransaction::Response(response)))).unwrap(),
          Err(e) => {
            trace_dbg!("Error: {}", e);
            tx.send(Action::Error(format!("Error: {}", e))).unwrap()
          },
        },
      };
      tx.send(Action::ExitProcessing).unwrap();
    });
  }

  pub fn process_response_handler(&mut self, transaction: ChatTransaction) {
    let tx = self.action_tx.clone().unwrap();
    if let ChatTransaction::StreamResponse(mut t) = transaction {
      if let Some(ChatTransaction::StreamResponse(r)) = self.transactions.last_mut() {
        r.choices.append(&mut t.choices);
      }
    } else {
      self.transactions.push(transaction);
    }
    tx.send(Action::Update).unwrap();
  }

  pub fn load_session_by_id(session_id: String) -> Session {
    Self::get_session_filepath(session_id.clone());
    let load_result = fs::read_to_string(Self::get_session_filepath(session_id.clone()));
    match load_result {
      Ok(session_data) => return serde_json::from_str(session_data.as_str()).unwrap(),
      Err(_) => {
        println!("Failed to load session data, creating new session");
        Session::new()
      },
    }
  }

  pub fn get_session_filepath(session_id: String) -> PathBuf {
    Path::new(SESSIONS_DIR).join(Self::get_session_filename(session_id))
  }

  pub fn get_session_filename(session_id: String) -> String {
    format!("{}.json", session_id)
  }

  pub fn get_last_session_file_path() -> Option<PathBuf> {
    ensure_directory_exists(SESSIONS_DIR).unwrap();
    let last_session_path = Path::new(SESSIONS_DIR).join("last_session.txt");
    if last_session_path.exists() {
      Some(fs::read_to_string(last_session_path).unwrap().into())
    } else {
      None
    }
  }

  pub fn load_last_session() -> Session {
    let last_session_path = Path::new(SESSIONS_DIR).join("last_session.txt");
    let last_session_id = fs::read_to_string(last_session_path).unwrap();
    Self::load_session_by_id(last_session_id)
  }

  fn save_session(&self) -> io::Result<()> {
    ensure_directory_exists(SESSIONS_DIR).unwrap();
    let session_file_path = Self::get_session_filepath(self.config.session_id.clone());
    let data = serde_json::to_string(&self)?;
    fs::write(session_file_path, data)?;
    Ok(())
  }

  pub fn save_last_session_id(&self) {
    ensure_directory_exists(SESSIONS_DIR).unwrap();
    let last_session_path = Path::new(SESSIONS_DIR).join("last_session.txt");
    fs::write(last_session_path, self.config.session_id.clone()).unwrap();
  }
}

pub async fn select_model(settings: &GPTSettings, client: Client<OpenAIConfig>) -> Result<Model, GPTConnectorError> {
  // Retrieve the list of available models
  let models_response = client.models().list().await;
  match models_response {
    Ok(response) => {
      let model_names: Vec<String> = response.data.iter().map(|model| model.id.clone()).collect();
      let available_models = ModelsList { default: GPT4.clone(), fallback: GPT3_TURBO.clone() };
      // Check if the default model is in the list
      if model_names.contains(&settings.default.name) {
        Ok(available_models.default)
      }
      // If not, check if the fallback model is in the list
      else if model_names.contains(&settings.fallback.name) {
        Ok(available_models.fallback)
      }
      // If neither is available, return an error
      else {
        Err(GPTConnectorError::Other("Neither the default nor the fallback model is accessible.".to_string()))
      }
    },
    Err(_) => Err(GPTConnectorError::Other("Failed to fetch the list of available models.".to_string())),
  }
}

pub fn create_openai_client() -> async_openai::Client<OpenAIConfig> {
  let api_key: String = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
  let openai_config = OpenAIConfig::new().with_api_key(api_key);
  let backoff = ExponentialBackoffBuilder::new() // Ensure backoff crate is added to Cargo.toml
    .with_max_elapsed_time(Some(std::time::Duration::from_secs(60)))
    .build();
  Client::with_config(openai_config).with_backoff(backoff)
}

pub fn construct_chat_completion_request_message(
  content: &str,
  model: &Model,
) -> Result<Vec<ChatCompletionRequestMessage>, GPTConnectorError> {
  let chunks = parse_input(content, CHUNK_TOKEN_LIMIT as usize, model.token_limit as usize).unwrap();

  let messages: Vec<ChatCompletionRequestMessage> = chunks
    .iter()
    .map(|chunk| ChatCompletionRequestMessage { role: Role::User, content: Some(chunk.clone()), ..Default::default() })
    .collect();
  Ok(messages)
}

pub fn construct_request(
  messages: Vec<ChatCompletionRequestMessage>,
  config: &SessionConfig, // model: Model,
                          // include_functions: bool,
) -> CreateChatCompletionRequest {
  let functions = match config.include_functions {
    true => Some(create_chat_completion_function_args(define_commands())),
    false => None,
  };
  CreateChatCompletionRequest {
    model: config.model.name.clone(),
    messages,
    functions,
    stream: Some(config.stream_response),
    ..Default::default()
  }
}

pub async fn create_embedding_request(
  model: &str,
  input: Vec<&str>,
) -> Result<CreateEmbeddingResponse, GPTConnectorError> {
  let client = Client::new();

  let request = CreateEmbeddingRequestArgs::default().model(model).input(input).build()?;

  let response = client.embeddings().create(request).await?;

  Ok(response)
}
