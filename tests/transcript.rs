use tempfile::TempDir;

use triadchat::room::transcript::{TranscriptEntry, TranscriptWriter};
use triadchat::state::{ChatMessage, MessageType, State};

#[test]
fn transcript_writer_appends_jsonl_entries() {
    let base = TempDir::new().unwrap();
    let mut writer = TranscriptWriter::open_with_base(base.path(), "room-1").unwrap();

    writer
        .append(&TranscriptEntry::chat("room-1", "takuro", "human", "chat", "この関数重い"))
        .unwrap();
    writer
        .append(&TranscriptEntry::chat("room-1", "ops-ai", "ai", "skill", "review-auth finished"))
        .unwrap();

    let path = base.path().join("triadchat/transcripts/room-1.jsonl");
    let raw = std::fs::read_to_string(path).unwrap();
    let entries = raw
        .lines()
        .map(|line| serde_json::from_str::<TranscriptEntry>(line).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].sender_id, "takuro");
    assert_eq!(entries[1].kind, "skill");
}

#[test]
fn transcript_writer_creates_directories_automatically() {
    let base = TempDir::new().unwrap();

    let writer = TranscriptWriter::open_with_base(base.path(), "room-2").unwrap();

    assert!(base.path().join("triadchat/transcripts").exists());
    drop(writer);
}

#[test]
fn transcript_writer_routes_entries_to_room_specific_files() {
    let base = TempDir::new().unwrap();
    let room_a = TranscriptEntry::chat("room-a", "takuro", "human", "chat", "a");
    let room_b = TranscriptEntry::chat("room-b", "tanaka", "human", "chat", "b");

    TranscriptWriter::append_to_base(base.path(), &room_a).unwrap();
    TranscriptWriter::append_to_base(base.path(), &room_b).unwrap();

    assert!(base.path().join("triadchat/transcripts/room-a.jsonl").exists());
    assert!(base.path().join("triadchat/transcripts/room-b.jsonl").exists());
}

#[test]
fn state_reuses_open_transcript_writer_for_active_room() {
    let base = TempDir::new().unwrap();
    let mut state = State::default();
    state.set_local_user_name("takuro");
    state.set_transcript_base_dir(Some(base.path().to_path_buf()));

    state.add_message(ChatMessage::new("takuro".into(), MessageType::Text("first".into())));

    let path = base.path().join("triadchat/transcripts/solo-takuro.jsonl");
    let renamed_path = base.path().join("triadchat/transcripts/solo-takuro-renamed.jsonl");
    std::fs::rename(&path, &renamed_path).unwrap();

    state.add_message(ChatMessage::new("takuro".into(), MessageType::Text("second".into())));
    drop(state);

    assert!(
        !path.exists(),
        "writer should keep using the open handle instead of reopening by path"
    );

    let raw = std::fs::read_to_string(renamed_path).unwrap();
    let entries = raw
        .lines()
        .map(|line| serde_json::from_str::<TranscriptEntry>(line).unwrap())
        .collect::<Vec<_>>();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].text, "first");
    assert_eq!(entries[1].text, "second");
}
