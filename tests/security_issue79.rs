use triadchat::application::Application;
use triadchat::config::Config;
use triadchat::message::Chunk;
use triadchat::room::transcript::TranscriptWriter;

#[test]
fn test_path_traversal_prevention() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    let download_dir = std::env::temp_dir().join("triadchat/downloads").join("sender_traversal");
    let _ = std::fs::remove_dir_all(&download_dir);

    // 1. Traverse filename
    app.inject_receive_chunk_for_test(
        "../../../traversal.txt",
        Chunk::Data(b"traversal".to_vec()),
        "sender_traversal",
    );
    app.inject_receive_chunk_for_test("../../../traversal.txt", Chunk::End, "sender_traversal");

    // 2. Windows separators
    app.inject_receive_chunk_for_test(
        "foo\\bar\\win_traversal.txt",
        Chunk::Data(b"win".to_vec()),
        "sender_traversal",
    );
    app.inject_receive_chunk_for_test(
        "foo\\bar\\win_traversal.txt",
        Chunk::End,
        "sender_traversal",
    );

    // 3. Absolute path
    app.inject_receive_chunk_for_test(
        "/etc/passwd",
        Chunk::Data(b"absolute".to_vec()),
        "sender_traversal",
    );
    app.inject_receive_chunk_for_test("/etc/passwd", Chunk::End, "sender_traversal");

    assert!(download_dir.join("traversal.txt").exists());
    assert!(download_dir.join("win_traversal.txt").exists());
    assert!(download_dir.join("passwd").exists());

    // Clean up
    let _ = std::fs::remove_dir_all(&download_dir);
}

#[test]
fn test_atomic_collision_resolution() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();

    let download_dir = std::env::temp_dir().join("triadchat/downloads").join("sender_collision");
    let _ = std::fs::remove_dir_all(&download_dir);

    // Transfer file first time
    app.inject_receive_chunk_for_test(
        "collision.txt",
        Chunk::Data(b"first".to_vec()),
        "sender_collision",
    );
    app.inject_receive_chunk_for_test("collision.txt", Chunk::End, "sender_collision");

    // Transfer same file second time
    app.inject_receive_chunk_for_test(
        "collision.txt",
        Chunk::Data(b"second".to_vec()),
        "sender_collision",
    );
    app.inject_receive_chunk_for_test("collision.txt", Chunk::End, "sender_collision");

    assert!(download_dir.join("collision.txt").exists());
    assert!(download_dir.join("collision_1.txt").exists());

    assert_eq!(std::fs::read_to_string(download_dir.join("collision.txt")).unwrap(), "first");
    assert_eq!(std::fs::read_to_string(download_dir.join("collision_1.txt")).unwrap(), "second");

    let _ = std::fs::remove_dir_all(&download_dir);
}

#[test]
fn test_room_id_sanitization_negative() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Attempt directory traversal room ID
    let traversal_room = "../../../evil_room";
    let writer_res = TranscriptWriter::open_with_base(temp_dir.path(), traversal_room);
    assert!(writer_res.is_ok());
    let writer = writer_res.unwrap();

    // Check that it wrote to temp_dir/triadchat/transcripts/evil_room.jsonl
    let expected_path = temp_dir.path().join("triadchat/transcripts").join("evil_room.jsonl");
    assert_eq!(writer.path(), expected_path);
    assert!(
        expected_path.exists()
            || !temp_dir.path().parent().unwrap().join("evil_room.jsonl").exists()
    );
}
