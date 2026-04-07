pub mod classifier;
pub mod parser;
pub mod prompt;
pub mod sidecar;
pub mod trigger;

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use crate::config::{AiConfig, LanguageConfig};
use crate::message::AiPayload;

use self::parser::parse_ai_payload;
use self::prompt::{
    companion_prompt, decisions_prompt, intervene_prompt, mention_prompt, summary_prompt,
    todos_prompt,
};
use self::sidecar::SidecarAdapter;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiTask {
    Summary,
    Todos,
    Decisions,
    Intervene,
    Companion,
    /// Direct reply to an `@ops-ai` mention. `last_messages[0]` is the verbatim message.
    Mention,
}

#[derive(Clone)]
pub struct AiMediator {
    sidecar: Arc<SidecarAdapter>,
    language: LanguageConfig,
}

impl AiMediator {
    pub fn new(workspace: &Path, config: &AiConfig, language: &LanguageConfig) -> Result<Self> {
        Ok(Self {
            sidecar: Arc::new(SidecarAdapter::new(workspace, config)?),
            language: language.clone(),
        })
    }

    pub async fn request(
        &self,
        task: AiTask,
        transcript: &str,
        last_messages: &[String],
    ) -> Result<AiPayload> {
        let prompt = match task {
            AiTask::Summary => summary_prompt(transcript, &self.language.ai_output),
            AiTask::Todos => todos_prompt(transcript, &self.language.ai_output),
            AiTask::Decisions => decisions_prompt(transcript, &self.language.ai_output),
            AiTask::Intervene => {
                intervene_prompt(transcript, last_messages, &self.language.ai_output)
            }
            AiTask::Companion => {
                companion_prompt(transcript, last_messages, &self.language.ai_output)
            }
            AiTask::Mention => {
                let message = last_messages.first().map(String::as_str).unwrap_or("");
                mention_prompt(message, transcript, &self.language.ai_output)
            }
        };
        let raw = self.sidecar.ask(&prompt).await?;
        Ok(parse_ai_payload(&raw))
    }

    pub async fn run_skill(&self, skill_name: &str, args: &[String]) -> Result<String> {
        self.sidecar.run_skill(skill_name, args).await
    }
}
