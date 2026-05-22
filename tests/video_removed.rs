use std::fs;
use std::path::Path;

#[test]
fn video_streaming_code_and_dependency_are_removed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let cargo_toml = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(!cargo_toml.contains("resize ="), "resize dependency should be removed");

    let message_rs = fs::read_to_string(root.join("src/message.rs")).unwrap();
    assert!(
        !message_rs.contains("Stream("),
        "NetMessage::Stream should be removed with video streaming"
    );

    let ui_rs = fs::read_to_string(root.join("src/ui.rs")).unwrap();
    assert!(!ui_rs.contains("draw_video_panel"), "video rendering panel should be removed");
}
