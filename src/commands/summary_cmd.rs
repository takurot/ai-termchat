use crate::commands::{AppCommand, Command, ParsedCommand, SummaryCommandKind};
use crate::util::Result;

pub struct SummaryCommand {
    name: &'static str,
    kind: SummaryCommandKind,
}

impl SummaryCommand {
    pub fn summary() -> Self {
        Self { name: "summary", kind: SummaryCommandKind::Summary }
    }

    pub fn todos() -> Self {
        Self { name: "todos", kind: SummaryCommandKind::Todos }
    }

    pub fn decisions() -> Self {
        Self { name: "decisions", kind: SummaryCommandKind::Decisions }
    }

    pub fn context() -> Self {
        Self { name: "context", kind: SummaryCommandKind::Context }
    }
}

impl Command for SummaryCommand {
    fn name(&self) -> &'static str {
        self.name
    }

    fn parse_params(&self, _params: Vec<String>) -> Result<ParsedCommand> {
        Ok(ParsedCommand::App(AppCommand::Summary(self.kind.clone())))
    }
}
