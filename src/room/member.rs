use crate::state::AiMode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemberKind {
    Human,
    Ai,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub id: String,
    pub kind: MemberKind,
    pub ai_mode: Option<AiMode>,
}

impl Member {
    pub fn human(id: impl Into<String>) -> Self {
        Self { id: id.into(), kind: MemberKind::Human, ai_mode: None }
    }

    pub fn ai(id: impl Into<String>, ai_mode: AiMode) -> Self {
        Self { id: id.into(), kind: MemberKind::Ai, ai_mode: Some(ai_mode) }
    }
}
