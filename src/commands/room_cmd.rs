use crate::commands::{AppCommand, Command, ParsedCommand};
use crate::state::AiMode;
use crate::util::Result;

pub struct RoomCommand;
pub struct PeersCommand;

impl Command for RoomCommand {
    fn name(&self) -> &'static str {
        "room"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let subcommand = params.first().map(String::as_str).unwrap_or("list");
        match subcommand {
            "create" => {
                let mut peers = Vec::new();
                let mut ai_mode = None;
                let mut index = 1;
                while index < params.len() {
                    match params[index].as_str() {
                        value if value.starts_with('@') => peers.push(value.trim_start_matches('@').to_string()),
                        "--ai" => {
                            let value = params
                                .get(index + 1)
                                .ok_or_else(|| anyhow::anyhow!("missing value for --ai"))?;
                            ai_mode = Some(parse_mode(value)?);
                            index += 1;
                        }
                        other => return Err(anyhow::anyhow!("unknown room argument: {other}")),
                    }
                    index += 1;
                }

                if peers.is_empty() {
                    return Err(anyhow::anyhow!("usage: /room create @user1 [@user2] [--ai <mode>]"));
                }

                Ok(ParsedCommand::App(AppCommand::RoomCreate { peers, ai_mode }))
            }
            "list" => Ok(ParsedCommand::App(AppCommand::RoomList)),
            "switch" => {
                let room_id = params
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("usage: /room switch <room_id>"))?;
                Ok(ParsedCommand::App(AppCommand::RoomSwitch(room_id.clone())))
            }
            other => Err(anyhow::anyhow!("unknown room command: {other}")),
        }
    }
}

impl Command for PeersCommand {
    fn name(&self) -> &'static str {
        "peers"
    }

    fn parse_params(&self, _params: Vec<String>) -> Result<ParsedCommand> {
        Ok(ParsedCommand::App(AppCommand::Peers))
    }
}

fn parse_mode(value: &str) -> Result<AiMode> {
    match value {
        "clerk" => Ok(AiMode::Clerk),
        "listener" => Ok(AiMode::Listener),
        "moderator" => Ok(AiMode::Moderator),
        "operator" => Ok(AiMode::Operator),
        other => Err(anyhow::anyhow!("unknown ai mode: {other}")),
    }
}
