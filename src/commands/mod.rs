pub mod ai_cmd;
pub mod art_cmd;
pub mod avatar_cmd;
pub mod peer_cmd;
pub mod room_cmd;
pub mod send_file;
pub mod skill_cmd;
pub mod summary_cmd;

use std::collections::HashMap;

use crate::action::Action;
use crate::config::AiProvider;
use crate::state::{AiFrequency, AiMode};
use crate::util::Result;

pub use ai_cmd::AiCommand;

/// Subcommands for the `/avatar` command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AvatarCommandKind {
    /// `/avatar set <target> <preset>` — change a target's avatar preset.
    Set { target: String, preset: String },
    /// `/avatar preview` — show current avatar in all sizes.
    Preview,
    /// `/avatar mode <compact|normal|expressive>` — change global size.
    Mode(String),
    /// `/avatar list` — list available presets.
    List,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SummaryCommandKind {
    Summary,
    Todos,
    Decisions,
    Context,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppCommand {
    Summary(SummaryCommandKind),
    SetAiMode(AiMode),
    SetAiQuiet(bool),
    SetAiFrequency(AiFrequency),
    SetAiProvider(AiProvider),
    RoomCreate { peers: Vec<String>, ai_mode: Option<AiMode> },
    RoomList,
    RoomSwitch(String),
    PeerConnect(String),
    Peers,
    TrustList,
    TrustAdd(String),
    TrustRemove(String),
    Skills,
    Skill { name: String, args: Vec<String> },
    RunProposal(usize),
    Cancel,
    Avatar(AvatarCommandKind),
    ArtList,
    ArtReload,
    Help,
}

pub enum ParsedCommand {
    Action(Box<dyn Action>),
    App(AppCommand),
}

pub trait Command {
    fn name(&self) -> &'static str;
    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand>;
}

#[derive(Default)]
pub struct CommandManager {
    parsers: HashMap<&'static str, Box<dyn Command + Send>>,
}

impl CommandManager {
    pub const COMMAND_PREFIX: &'static str = "/";

    pub fn with(mut self, command_parser: impl Command + 'static + Send) -> Self {
        self.parsers.insert(command_parser.name(), Box::new(command_parser));
        self
    }

    pub fn find_command(&self, input: &str) -> Option<Result<ParsedCommand>> {
        let input = input.strip_prefix(Self::COMMAND_PREFIX)?;
        let mut input = input.splitn(2, char::is_whitespace);
        let first = input.next()?;
        if first == "help" {
            return Some(Ok(ParsedCommand::App(AppCommand::Help)));
        }

        let parser = self.parsers.get(first)?;
        let param_str = input.next().unwrap_or("");
        Some(
            shellwords::split(param_str)
                .map_err(Into::into)
                .and_then(|params| parser.parse_params(params)),
        )
    }
}
