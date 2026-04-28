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
                        value if value.starts_with('@') => {
                            peers.push(value.trim_start_matches('@').to_string())
                        }
                        "--ai" => {
                            let value = params
                                .get(index + 1)
                                .ok_or_else(|| anyhow::anyhow!("missing value for --ai"))?;
                            ai_mode = Some(value.parse::<AiMode>()?);
                            index += 1;
                        }
                        other => return Err(anyhow::anyhow!("unknown room argument: {other}")),
                    }
                    index += 1;
                }

                if peers.is_empty() {
                    return Err(anyhow::anyhow!(
                        "usage: /room create @user1 [@user2] [--ai <mode>]"
                    ));
                }

                Ok(ParsedCommand::App(AppCommand::RoomCreate { peers, ai_mode }))
            }
            "list" => Ok(ParsedCommand::App(AppCommand::RoomList)),
            "switch" => {
                let room_id = params
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("usage: /room switch <room_id|index>"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{Command, ParsedCommand};

    #[test]
    fn room_create_accepts_companion_ai_mode() {
        let parsed = RoomCommand
            .parse_params(vec!["create".into(), "@user".into(), "--ai".into(), "companion".into()])
            .expect("companion is a valid AI mode");

        assert!(matches!(
            parsed,
            ParsedCommand::App(AppCommand::RoomCreate { ai_mode: Some(AiMode::Companion), .. })
        ));
    }
}
