use crate::action::{Action, Processing};
use crate::commands::{Command, ParsedCommand};
use crate::encoder::Encoder;
use crate::message::{Chunk, NetMessage};
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
    encoder: Encoder,
    failed_endpoints: HashSet<Endpoint>,
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
            encoder: Encoder::new(),
            failed_endpoints: HashSet::new(),
        })
    }
}

impl Action for SendFile {
    fn process(&mut self, state: &mut State, network: &NetworkController) -> Processing {
        if self.progress_id.is_none() {
            let id = state.add_progress_message(&self.file_name, self.file_size);
            self.progress_id = Some(id);
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
        let message = self.encoder.encode(net_message);

        let endpoints = state
            .all_user_endpoints()
            .into_iter()
            .filter(|e| !self.failed_endpoints.contains(e))
            .collect::<Vec<_>>();

        let result = crate::util::send_all(network, &endpoints, message);
        if let Err(ref errors) = result {
            for (endpoint, _) in errors {
                self.failed_endpoints.insert(*endpoint);
            }
        }
        result.report_if_err(state);

        processing
    }
}
