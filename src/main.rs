use async_openai::types::Role;
use clap::Parser;
use rustyline::error::ReadlineError;
use sazid::gpt_connector::GPTConnector;
use async_openai::types::ChatCompletionRequestMessage;
use sazid::session_manager::SessionManager;
use sazid::ui::UI;
use std::ffi::OsString;
use std::path::PathBuf;

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

    #[clap(
        short = 'i',
        long,
        value_name = "PATH",
        help = "Import a file or directory for GPT to process"
    )]
    ingest: Option<OsString>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let gpt = GPTConnector::new();
    let session_manager = SessionManager::new(PathBuf::from("./"));

    if let Some(path) = &opts.ingest {
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()?
            .block_on(session_manager.handle_ingest(&path.to_string_lossy().to_string()))?;
    }

    UI::display_startup_message();

    let mut messages: Vec<ChatCompletionRequestMessage> = if !opts.new {
        match opts.continue_session {
            Some(session_file) => session_manager.load_session(&session_file)?,
            None => {
                if let Some(last_session) = session_manager.load_last_session_filename() {
                    session_manager.load_session(&last_session)?
                } else {
                    vec![]
                }
            }
        }
    } else {
        vec![]
    };

    for message in &messages {
        UI::display_message(message.role.clone(), &message.content.unwrap_or_default());
    }

    loop {
        match UI::read_input("You: ") {
            Ok(input) => {
                let input = input.trim();

                if input.starts_with("ingest ") {
                    let filepath = input.split_whitespace().nth(1).unwrap_or_default();
                    tokio::runtime::Builder::new_current_thread()
                        .enable_io()
                        .enable_time()
                        .build()?
                        .block_on(session_manager.handle_ingest(&filepath.to_string()))?;
                } else {
                    if input == "exit" || input == "quit" {
                        let session_filename = session_manager.new_session_filename();
                        session_manager.save_session(&session_filename, &messages)?;
                        session_manager.save_last_session_filename(&session_filename)?;
                        UI::display_exit_message();
                        break;
                    }
                    let user_message = ChatCompletionRequestMessage {
                        role: Role::User,
                        content: Some(input.to_string()),
                        function_call: None,  // If you have appropriate data, replace None
                        name: None,           // If you have appropriate data, replace None
                    };
                    messages.push(user_message.clone());

                    match tokio::runtime::Builder::new_current_thread()
                        .enable_io()
                        .enable_time()
                        .build()?
                        .block_on(gpt.send_request(vec![input.to_string()]))
                        {
                        Ok(response) => {
                            for choice in &response.choices {
                                UI::display_message(choice.message.role, &choice.message.content.unwrap_or_default());
                            }
                        }
                        Err(e) => {
                            println!("Error sending request to GPT: {:?}", e);
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                let session_filename = session_manager.new_session_filename();
                session_manager.save_session(&session_filename, &messages)?;
                session_manager.save_last_session_filename(&session_filename)?;
                UI::display_exit_message();
                break;
            }
            Err(ReadlineError::Eof) => {
                let session_filename = session_manager.new_session_filename();
                session_manager.save_session(&session_filename, &messages)?;
                session_manager.save_last_session_filename(&session_filename)?;
                UI::display_exit_message();
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}
