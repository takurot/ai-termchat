use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use tempfile::TempDir;

use triadchat::ai::sidecar::SidecarAdapter;
use triadchat::config::AiConfig;

fn write_script(dir: &TempDir, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

#[tokio::test]
async fn sidecar_returns_stdout() {
    let dir = TempDir::new().unwrap();
    let script = write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: mock summary\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );
    let adapter = SidecarAdapter::from_command(dir.path(), script, Duration::from_secs(1))
        .expect("adapter should be created");

    let output = adapter.ask("hello").await.expect("sidecar should succeed");
    assert!(output.contains("mock summary"));
}

#[tokio::test]
async fn sidecar_times_out() {
    let dir = TempDir::new().unwrap();
    let script = write_script(&dir, "slow-claude.sh", "#!/bin/sh\nsleep 2\nprintf 'late'\n");
    let adapter = SidecarAdapter::from_command(dir.path(), script, Duration::from_millis(100))
        .expect("adapter should be created");

    let error = adapter.ask("hello").await.expect_err("sidecar should time out");
    assert!(error.to_string().contains("timed out"));
}

#[test]
fn configured_sidecar_command_can_be_resolved_from_path() {
    let dir = TempDir::new().unwrap();
    write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\nprintf 'INTENT: Summary\nTEXT: mock summary\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\n'",
    );

    let original_path = std::env::var_os("PATH");
    let mut new_path = dir.path().as_os_str().to_os_string();
    if let Some(path) = original_path.as_ref() {
        new_path.push(":");
        new_path.push(path);
    }
    std::env::set_var("PATH", &new_path);

    let config =
        AiConfig { enabled: true, command: Some("mock-claude.sh".into()), timeout_secs: 1 };
    let adapter = SidecarAdapter::new(dir.path(), &config);

    if let Some(path) = original_path {
        std::env::set_var("PATH", path);
    } else {
        std::env::remove_var("PATH");
    }

    assert!(adapter.is_ok());
}
