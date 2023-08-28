mod gpt_connector;
mod logger;
mod session_manager;
mod ui;

use rustyline::error::ReadlineError;
use clap::Parser;
use gpt_connector::{ChatCompletionRequestMessage, GPTConnector};
use logger::Logger;
use session_manager::SessionManager;
use ui::UI;
use async_openai::types::Role;

#[derive(Parser)]
#[clap(
    version = "1.0",
    author = "Your Name",
    about = "Interactive chat with GPT"
)]
struct Opts {
    #[clap(short = 'n', long, help = "Start a new chat session")]
    new: bool,
    
    #[clap(short = 'c', long, help = "Continue from a specified session file")]
    continue_session: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let gpt = GPTConnector::new();
    let logger = Logger::new();

    UI::display_startup_message();

    let mut messages: Vec<ChatCompletionRequestMessage> = if !opts.new {
        match opts.continue_session {
            Some(ref session_file) => {
                SessionManager::load_session(session_file)?
            },
            None => {
                if let Some(last_session) = SessionManager::load_last_session_filename() {
                    SessionManager::load_session(&last_session)?
                } else {
                    vec![]
                }
            }
        }
    } else {
        vec![]
    };

    for message in &messages {
        UI::display_message(message.role.clone(), &message.content);
    }

    loop {
        match UI::read_input("You: ") {
            Ok(input) => {
                let input = input.trim();

                if input == "exit" || input == "quit" {
                    let session_filename = SessionManager::new_session_filename();
                    SessionManager::save_session(&session_filename, &messages)?;
                    SessionManager::save_last_session_filename(&session_filename)?;
                    UI::display_exit_message();
                    break;
                }

                let user_message = ChatCompletionRequestMessage {
                    role: Role::User,
                    content: input.to_string(),
                };
                messages.push(user_message.clone());

                let response = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()?
                    .block_on(gpt.send_request(messages.clone()))?;

                let assistant_message = ChatCompletionRequestMessage {
                    role: response.role.clone(),
                    content: response.content.clone(),
                };
                messages.push(assistant_message.clone());

                logger.log_interaction(&user_message.content, &response.content);
                UI::display_message(response.role, &response.content);
            },
            Err(ReadlineError::Interrupted) => {
                let session_filename = SessionManager::new_session_filename();
                SessionManager::save_session(&session_filename, &messages)?;
                SessionManager::save_last_session_filename(&session_filename)?;
                UI::display_exit_message();
                break;
            },
            Err(ReadlineError::Eof) => {
                let session_filename = SessionManager::new_session_filename();
                SessionManager::save_session(&session_filename, &messages)?;
                SessionManager::save_last_session_filename(&session_filename)?;
                UI::display_exit_message();
                break;
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}
