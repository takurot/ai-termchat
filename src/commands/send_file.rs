use crate::action::{Action, Processing};
use crate::commands::{Command, ParsedCommand};
use crate::message::{Chunk, NetMessage};
use crate::secure::{send_secure_to_endpoints, SecureState};
use crate::state::State;
use crate::util::{Reportable, Result};

use std::collections::HashSet;
use message_io::network::{Endpoint, NetworkController};

use std::io::Read;
use std::path::Path;
use std::time::Duration;

pub struct SendFileCommand;

impl Command for SendFileCommand {
    fn name(&self) -> &'static str {
        "send"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let param = params.first().ok_or_else(|| anyhow::anyhow!("No file specified"))?;
        let file_path = shellexpand::full(param)?;
        match SendFile::new(&file_path) {
            Ok(action) => Ok(ParsedCommand::Action(Box::new(action))),
            Err(e) => Err(e),
        }
    }
}

pub struct SendFile {
    file: std::fs::File,
    file_name: String,
    file_size: u64,
    progress_id: Option<usize>,
    failed_endpoints: HashSet<Endpoint>,
    offered: bool,
    accepted: bool,
}

impl SendFile {
    const CHUNK_SIZE: usize = 32768;

    pub fn new(file_path: &str) -> Result<SendFile> {
        const READ_FILENAME_ERROR: &str = "Unable to read file name";
        let file_path = Path::new(file_path);
        let file_name = file_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!(READ_FILENAME_ERROR))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!(READ_FILENAME_ERROR))?
            .to_string();

        let file_size = std::fs::metadata(file_path)?.len();
        let file = std::fs::File::open(file_path)?;

        Ok(SendFile {
            file,
            file_name,
            file_size,
            progress_id: None,
            failed_endpoints: HashSet::new(),
            offered: false,
            accepted: false,
        })
    }
}

impl Action for SendFile {
    fn process(
        &mut self,
        state: &mut State,
        network: &NetworkController,
        secure: &mut SecureState,
    ) -> Processing {
        if self.progress_id.is_none() {
            let id = state.add_progress_message(&self.file_name, self.file_size);
            self.progress_id = Some(id);
        }

        if !self.offered {
            self.offered = true;
            let user_name = state.local_user_name().to_string();
            let offer = NetMessage::TransferOffer {
                file_name: self.file_name.clone(),
                file_size: self.file_size,
                sender: user_name,
            };
            let endpoints: Vec<Endpoint> = state
                .all_user_endpoints()
                .into_iter()
                .filter(|e| !self.failed_endpoints.contains(e))
                .collect();
            let result = send_secure_to_endpoints(network, secure, &endpoints, &offer);
            if result.is_err() {
                result.report_if_err(state);
                return Processing::Completed;
            }
            return Processing::Partial(Duration::from_millis(50));
        }

        if !self.accepted {
            if state.has_accepted_outbound_transfer(&self.file_name) {
                self.accepted = true;
            } else {
                return Processing::Partial(Duration::from_millis(100));
            }
        }

        let mut data = [0; Self::CHUNK_SIZE];
        let (bytes_read, chunk, processing) = match self.file.read(&mut data) {
            Ok(0) => (0, Chunk::End, Processing::Completed),
            Ok(bytes_read) => {
                // We add a minor delay to introduce a rate in the sending.
                let processing = Processing::Partial(Duration::from_micros(100));
                (bytes_read, Chunk::Data(data[..bytes_read].to_vec()), processing)
            }
            Err(error) => {
                format!("Error sending file. error: {}", error).report_err(state);
                (0, Chunk::Error, Processing::Completed)
            }
        };

        if let Some(id) = self.progress_id {
            state.progress_message_update(id, bytes_read as u64);
        }

        let net_message = NetMessage::UserData(self.file_name.clone(), chunk);

        let endpoints = state
            .all_user_endpoints()
            .into_iter()
            .filter(|e| !self.failed_endpoints.contains(e))
            .collect::<Vec<_>>();

        let result = send_secure_to_endpoints(network, secure, &endpoints, &net_message);
        if let Err(ref errors) = result {
            for (endpoint, _) in errors {
                self.failed_endpoints.insert(*endpoint);
            }
        }
        result.report_if_err(state);

        processing
    }
}
