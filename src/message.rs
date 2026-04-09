use rgb::RGB8;
use serde::{Deserialize, Serialize};

use crate::state::AiMode;

pub type RoomId = String;
pub type MemberId = String;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    pub text: String,
    pub assignee: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredOutput {
    pub todos: Vec<TodoItem>,
    pub decisions: Vec<String>,
    pub skill_suggestions: Vec<String>,
    pub raw_text: Option<String>,
}

impl StructuredOutput {
    pub fn empty() -> Self {
        Self {
            todos: Vec::new(),
            decisions: Vec::new(),
            skill_suggestions: Vec::new(),
            raw_text: None,
        }
    }

    pub fn raw(raw: impl Into<String>) -> Self {
        Self { raw_text: Some(raw.into()), ..Self::empty() }
    }

    pub fn is_empty(&self) -> bool {
        self.todos.is_empty()
            && self.decisions.is_empty()
            && self.skill_suggestions.is_empty()
            && self.raw_text.is_none()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiIntent {
    #[default]
    Clarify,
    Summary,
    Todo,
    Decision,
    SkillSuggest,
    Skip,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPayload {
    pub text: String,
    pub intent: AiIntent,
    pub structured: Option<StructuredOutput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillResultPayload {
    pub skill_name: String,
    pub summary: String,
    pub success: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub user_name: String,
    pub server_port: u16,
    pub node_version: String,
    #[serde(default)]
    pub avatar: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Chunk {
    Data(Vec<u8>),
    Error,
    End,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    HelloLan(String, u16),
    HelloUser(String),
    UserMessage(String),
    UserData(String, Chunk),
    Stream(Option<(Vec<RGB8>, usize, usize)>),
    AiMessage(AiPayload),
    PeerInfo(PeerInfo),
    RoomCreate(RoomId, Vec<MemberId>, Option<AiMode>),
    RoomJoin(RoomId),
    SkillResult(SkillResultPayload),
}
