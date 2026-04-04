use crate::commands::{AppCommand, Command, ParsedCommand};
use crate::state::{AiFrequency, AiMode};
use crate::util::Result;

pub struct AiCommand;

impl Command for AiCommand {
    fn name(&self) -> &'static str {
        "ai"
    }

    fn parse_params(&self, params: Vec<String>) -> Result<ParsedCommand> {
        let subcommand = params.first().map(String::as_str).unwrap_or("help");
        match subcommand {
            "mode" => {
                let mode = parse_mode(params.get(1).map(String::as_str).unwrap_or("clerk"))?;
                Ok(ParsedCommand::App(AppCommand::SetAiMode(mode)))
            }
            "quiet" => {
                let enabled = match params.get(1).map(String::as_str).unwrap_or("on") {
                    "on" => true,
                    "off" => false,
                    other => return Err(anyhow::anyhow!("unknown quiet value: {other}")),
                };
                Ok(ParsedCommand::App(AppCommand::SetAiQuiet(enabled)))
            }
            "freq" => {
                let frequency =
                    parse_frequency(params.get(1).map(String::as_str).unwrap_or("normal"))?;
                Ok(ParsedCommand::App(AppCommand::SetAiFrequency(frequency)))
            }
            _ => Err(anyhow::anyhow!(
                "usage: /ai mode <clerk|listener|moderator|operator> | /ai quiet <on|off> | /ai freq <low|normal|high>"
            )),
        }
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

fn parse_frequency(value: &str) -> Result<AiFrequency> {
    match value {
        "low" => Ok(AiFrequency::Low),
        "normal" => Ok(AiFrequency::Normal),
        "high" => Ok(AiFrequency::High),
        other => Err(anyhow::anyhow!("unknown ai frequency: {other}")),
    }
}
