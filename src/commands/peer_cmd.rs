use crate::commands::{AppCommand, Command, ParsedCommand};
use crate::util::Result;

pub struct PeerCommand;
pub struct TrustCommand;

impl Command for PeerCommand {
    fn name(&self) -> &'static str {
        "peer"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        match params.first().map(String::as_str) {
            Some("connect") => {
                let target = params
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("usage: /peer connect <host:port>"))?;
                Ok(ParsedCommand::App(AppCommand::PeerConnect(target.clone())))
            }
            Some(other) => Err(anyhow::anyhow!("unknown peer command: {other}")),
            None => Err(anyhow::anyhow!("usage: /peer connect <host:port>")),
        }
    }
}

impl Command for TrustCommand {
    fn name(&self) -> &'static str {
        "trust"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        match params.first().map(String::as_str).unwrap_or("list") {
            "list" => Ok(ParsedCommand::App(AppCommand::TrustList)),
            "add" => {
                let target = params
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("usage: /trust add <peer|fingerprint>"))?;
                Ok(ParsedCommand::App(AppCommand::TrustAdd(target.clone())))
            }
            "remove" => {
                let target = params
                    .get(1)
                    .ok_or_else(|| anyhow::anyhow!("usage: /trust remove <peer|fingerprint>"))?;
                Ok(ParsedCommand::App(AppCommand::TrustRemove(target.clone())))
            }
            other => Err(anyhow::anyhow!("unknown trust command: {other}")),
        }
    }
}
