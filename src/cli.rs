use crate::commands::TryCommand;
use bevy::prelude::*;
use rustyline::{DefaultEditor, error::ReadlineError};
use std::sync::mpsc::{self, Receiver};

pub fn receive_cli_inputs(mut commands: Commands, mut receiver: Local<Option<Receiver<String>>>) {
    match receiver.as_mut() {
        Some(rx) => {
            while let Ok(input) = rx.try_recv() {
                commands.trigger(TryCommand::from_entry(input))
            }
        }
        None => {
            let (tx, rx) = mpsc::channel::<String>();
            *receiver = Some(rx);
            std::thread::spawn(move || {
                let mut rl = DefaultEditor::new().expect("Failed to create editor");

                loop {
                    match rl.readline("") {
                        Ok(line) => {
                            let trimmed = line.trim().to_string();
                            if trimmed.is_empty() {
                                continue;
                            }
                            rl.add_history_entry(&trimmed).ok();
                            if tx.send(trimmed).is_err() {
                                break; // Bevy shut down, receiver dropped
                            }
                        }
                        Err(ReadlineError::Interrupted) => {
                            // Ctrl+C — optionally send a quit command
                            tx.send("quit".to_string()).ok();
                            break;
                        }
                        Err(ReadlineError::Eof) => {
                            // Ctrl+D — treat as quit
                            tx.send("quit".to_string()).ok();
                            break;
                        }
                        Err(e) => {
                            error!("CLI error: {e}");
                            break;
                        }
                    }
                }
            });
        }
    }
}
