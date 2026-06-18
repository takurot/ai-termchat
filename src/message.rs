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
    AiMessage(AiPayload),
    PeerInfo(PeerInfo),
    RoomCreate(RoomId, Vec<MemberId>),
    RoomCreateV2 { room_id: RoomId, members: Vec<MemberId>, ai_mode: Option<AiMode> },
    RoomJoin(RoomId),
    SkillResult(SkillResultPayload),
}

pub const MAX_NAME_LEN: usize = 256;
pub const MAX_CHAT_MESSAGE_LEN: usize = 65536; // 64 KB
pub const MAX_FILE_NAME_LEN: usize = 256;
pub const MAX_FILE_CHUNK_LEN: usize = 65536; // 64 KB
pub const MAX_AI_TEXT_LEN: usize = 524288; // 512 KB
pub const MAX_TODO_ITEMS: usize = 256;
pub const MAX_TODO_TEXT_LEN: usize = 4096;
pub const MAX_DECISIONS: usize = 256;
pub const MAX_DECISION_TEXT_LEN: usize = 4096;
pub const MAX_SKILL_SUGGESTIONS: usize = 256;
pub const MAX_SKILL_NAME_LEN: usize = 128;
pub const MAX_VERSION_LEN: usize = 64;
pub const MAX_AVATAR_LEN: usize = 65536; // 64 KB
pub const MAX_ROOM_ID_LEN: usize = 256;
pub const MAX_ROOM_MEMBERS: usize = 256;
pub const MAX_SKILL_SUMMARY_LEN: usize = 65536; // 64 KB

impl TodoItem {
    pub fn validate(&self) -> bool {
        self.text.len() <= MAX_TODO_TEXT_LEN
            && self.assignee.as_ref().is_none_or(|a| a.len() <= MAX_NAME_LEN)
    }
}

impl StructuredOutput {
    pub fn validate(&self) -> bool {
        self.todos.len() <= MAX_TODO_ITEMS
            && self.todos.iter().all(|t| t.validate())
            && self.decisions.len() <= MAX_DECISIONS
            && self.decisions.iter().all(|d| d.len() <= MAX_DECISION_TEXT_LEN)
            && self.skill_suggestions.len() <= MAX_SKILL_SUGGESTIONS
            && self.skill_suggestions.iter().all(|s| s.len() <= MAX_SKILL_NAME_LEN)
            && self.raw_text.as_ref().is_none_or(|r| r.len() <= MAX_AI_TEXT_LEN)
    }
}

impl AiPayload {
    pub fn validate(&self) -> bool {
        self.text.len() <= MAX_AI_TEXT_LEN
            && self.structured.as_ref().is_none_or(|s| s.validate())
    }
}

impl SkillResultPayload {
    pub fn validate(&self) -> bool {
        self.skill_name.len() <= MAX_SKILL_NAME_LEN && self.summary.len() <= MAX_SKILL_SUMMARY_LEN
    }
}

impl PeerInfo {
    pub fn validate(&self) -> bool {
        self.user_name.len() <= MAX_NAME_LEN
            && self.node_version.len() <= MAX_VERSION_LEN
            && self.avatar.len() <= MAX_AVATAR_LEN
    }
}

impl Chunk {
    pub fn validate(&self) -> bool {
        match self {
            Chunk::Data(data) => data.len() <= MAX_FILE_CHUNK_LEN,
            Chunk::Error | Chunk::End => true,
        }
    }
}

impl NetMessage {
    pub fn validate(&self) -> bool {
        match self {
            NetMessage::HelloLan(name, _) => name.len() <= MAX_NAME_LEN,
            NetMessage::HelloUser(name) => name.len() <= MAX_NAME_LEN,
            NetMessage::UserMessage(msg) => msg.len() <= MAX_CHAT_MESSAGE_LEN,
            NetMessage::UserData(filename, chunk) => {
                filename.len() <= MAX_FILE_NAME_LEN && chunk.validate()
            }
            NetMessage::AiMessage(payload) => payload.validate(),
            NetMessage::PeerInfo(info) => info.validate(),
            NetMessage::RoomCreate(room_id, members) => {
                room_id.len() <= MAX_ROOM_ID_LEN
                    && members.len() <= MAX_ROOM_MEMBERS
                    && members.iter().all(|m| m.len() <= MAX_NAME_LEN)
            }
            NetMessage::RoomCreateV2 { room_id, members, ai_mode: _ } => {
                room_id.len() <= MAX_ROOM_ID_LEN
                    && members.len() <= MAX_ROOM_MEMBERS
                    && members.iter().all(|m| m.len() <= MAX_NAME_LEN)
            }
            NetMessage::RoomJoin(room_id) => room_id.len() <= MAX_ROOM_ID_LEN,
            NetMessage::SkillResult(payload) => payload.validate(),
        }
    }
}
