use tempfile::TempDir;

use triadchat::room::transcript::{TranscriptEntry, TranscriptWriter};

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
