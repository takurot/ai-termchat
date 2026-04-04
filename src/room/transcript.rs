use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub id: String,
    pub room_id: String,
    pub sender_id: String,
    pub sender_type: String,
    pub text: String,
    pub kind: String,
    pub timestamp: String,
    pub intent: Option<String>,
    pub structured: Option<serde_json::Value>,
}

impl TranscriptEntry {
    pub fn chat(
        room_id: impl Into<String>,
        sender_id: impl Into<String>,
        sender_type: impl Into<String>,
        kind: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("msg-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()),
            room_id: room_id.into(),
            sender_id: sender_id.into(),
            sender_type: sender_type.into(),
            text: text.into(),
            kind: kind.into(),
            timestamp: chrono::Local::now().to_rfc3339(),
            intent: None,
            structured: None,
        }
    }

    pub fn ai(
        room_id: impl Into<String>,
        sender_id: impl Into<String>,
        text: impl Into<String>,
        intent: Option<String>,
        structured: Option<serde_json::Value>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("msg-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()),
            room_id: room_id.into(),
            sender_id: sender_id.into(),
            sender_type: "ai".into(),
            text: text.into(),
            kind: kind.into(),
            timestamp: chrono::Local::now().to_rfc3339(),
            intent,
            structured,
        }
    }
}

pub struct TranscriptWriter {
    file: File,
    path: PathBuf,
}

impl TranscriptWriter {
    pub fn open(room_id: &str) -> Result<Self> {
        let base = dirs_next::data_dir().ok_or_else(|| anyhow::anyhow!("no data dir"))?;
        Self::open_with_base(base, room_id)
    }

    pub fn open_with_base(base: impl AsRef<Path>, room_id: &str) -> Result<Self> {
        let dir = Self::transcript_dir(base);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{room_id}.jsonl"));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { file, path })
    }

    pub fn append(&mut self, entry: &TranscriptEntry) -> Result<()> {
        writeln!(self.file, "{}", serde_json::to_string(entry)?)?;
        self.file.flush()?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn transcript_dir(base: impl AsRef<Path>) -> PathBuf {
        base.as_ref().join("triadchat/transcripts")
    }
}
