use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tempfile::TempDir;

use triadchat::ai::sidecar::SidecarAdapter;
use triadchat::config::{AiConfig, AiProvider};

fn write_script(dir: &TempDir, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, body).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|error| error.into_inner())
}

#[tokio::test]
async fn sidecar_returns_stdout() {
    let dir = TempDir::new().unwrap();
    let script = write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\ncat <<'EOF'\nINTENT: Summary\nTEXT: mock summary\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\nEOF\n",
    );
    let adapter = SidecarAdapter::from_command(dir.path(), "/bin/sh", Duration::from_secs(3))
        .expect("adapter should be created");

    let (output, truncated) = adapter
        .ask(script.to_str().expect("script path should be utf-8"))
        .await
        .expect("sidecar should succeed");
    assert!(output.contains("mock summary"));
    assert!(!truncated);
}

#[tokio::test]
async fn sidecar_times_out() {
    let dir = TempDir::new().unwrap();
    let script = write_script(&dir, "slow-claude.sh", "#!/bin/sh\nsleep 2\nprintf 'late'\n");
    let adapter = SidecarAdapter::from_command(dir.path(), "/bin/sh", Duration::from_millis(100))
        .expect("adapter should be created");

    let error = adapter
        .ask(script.to_str().expect("script path should be utf-8"))
        .await
        .expect_err("sidecar should time out");
    assert!(error.to_string().contains("timed out"));
}

#[test]
fn configured_sidecar_command_can_be_resolved_from_path() {
    let _guard = env_lock();
    let dir = TempDir::new().unwrap();
    write_script(
        &dir,
        "mock-claude.sh",
        "#!/bin/sh\ncat <<'EOF'\nINTENT: Summary\nTEXT: mock summary\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\nEOF\n",
    );

    let original_path = std::env::var_os("PATH");
    let mut new_path = dir.path().as_os_str().to_os_string();
    if let Some(path) = original_path.as_ref() {
        new_path.push(":");
        new_path.push(path);
    }
    std::env::set_var("PATH", &new_path);

    let config = AiConfig {
        enabled: true,
        provider: AiProvider::Claude,
        command: Some("mock-claude.sh".into()),
        timeout_secs: 1,
    };
    let adapter = SidecarAdapter::new(dir.path(), &config);

    if let Some(path) = original_path {
        std::env::set_var("PATH", path);
    } else {
        std::env::remove_var("PATH");
    }

    assert!(adapter.is_ok());
}

#[test]
fn ai_config_without_provider_defaults_to_claude() {
    let config: AiConfig =
        toml::from_str("enabled = true\ntimeout_secs = 30").expect("config should parse");

    assert_eq!(config.provider, AiProvider::Claude);
}

#[test]
fn custom_provider_requires_command() {
    let dir = TempDir::new().unwrap();
    let config =
        AiConfig { enabled: true, provider: AiProvider::Custom, command: None, timeout_secs: 1 };

    let error = SidecarAdapter::new(dir.path(), &config).expect_err("custom without command fails");
    assert!(error.to_string().contains("requires"));
}

fn capture_args_script(dir: &TempDir, args_file: &std::path::Path) -> std::path::PathBuf {
    write_script(
        dir,
        "capture.sh",
        &format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf 'ok'\n", args_file.display()),
    )
}

#[tokio::test]
async fn provider_invocation_uses_provider_specific_args() {
    let dir = TempDir::new().unwrap();
    let args_file = dir.path().join("args.txt");
    let script = capture_args_script(&dir, &args_file);

    let cases = [
        (AiProvider::Claude, vec!["-p", "hello world"]),
        (AiProvider::Codex, vec!["exec", "hello world"]),
        (AiProvider::Gemini, vec!["-p", "hello world"]),
    ];

    for (provider, expected_args) in cases {
        let config = AiConfig {
            enabled: true,
            provider,
            command: Some(script.display().to_string()),
            timeout_secs: 3,
        };
        let adapter = SidecarAdapter::new(dir.path(), &config).expect("adapter should build");

        let (output, truncated) = adapter.ask("hello world").await.expect("ask should succeed");
        assert_eq!(output, "ok");
        assert!(!truncated);

        let captured = fs::read_to_string(&args_file).expect("args should be captured");
        let captured_args = captured.lines().collect::<Vec<_>>();
        assert_eq!(captured_args, expected_args);
    }
}
