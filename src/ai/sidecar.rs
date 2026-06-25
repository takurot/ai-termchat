use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tokio::process::Command;

use crate::config::{AiConfig, AiProvider};

#[derive(Clone, Debug)]
pub struct SidecarAdapter {
    workspace: PathBuf,
    command: PathBuf,
    prefix_args: Vec<String>,
    timeout: Duration,
}

impl SidecarAdapter {
    pub fn new(workspace: &Path, config: &AiConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_secs.max(1));
        let command = if let Some(command) = &config.command {
            resolve_command(command)?
        } else if let Some(default_command) = config.provider.default_command() {
            which::which(default_command)
                .with_context(|| format!("{default_command} command was not found in PATH"))?
        } else {
            bail!("ai provider 'custom' requires ai.command to be set");
        };
        Self::from_command_with_provider(workspace, command, config.provider.clone(), timeout)
    }

    pub fn from_command(
        workspace: &Path,
        command: impl Into<PathBuf>,
        timeout: Duration,
    ) -> Result<Self> {
        Self::from_command_with_provider(workspace, command, AiProvider::Claude, timeout)
    }

    fn from_command_with_provider(
        workspace: &Path,
        command: impl Into<PathBuf>,
        provider: AiProvider,
        timeout: Duration,
    ) -> Result<Self> {
        let command = command.into();
        if !command.exists() {
            bail!("sidecar command does not exist: {}", command.display());
        }
        Ok(Self {
            workspace: workspace.to_path_buf(),
            command,
            prefix_args: provider.prefix_args().iter().map(|arg| (*arg).to_string()).collect(),
            timeout,
        })
    }

    pub async fn ask(&self, prompt: &str) -> Result<(String, bool)> {
        self.run_prompt(prompt, self.timeout).await
    }

    pub async fn run_skill(&self, skill_name: &str, args: &[String]) -> Result<String> {
        let suffix = if args.is_empty() { String::new() } else { format!(" {}", args.join(" ")) };
        let (output, _) =
            self.run_prompt(&format!("/{skill_name}{suffix}"), Duration::from_secs(60)).await?;
        Ok(output)
    }

    async fn run_prompt(&self, prompt: &str, timeout: Duration) -> Result<(String, bool)> {
        let (prompt, truncated) = truncate_prompt(prompt, 50_000);
        let mut command = Command::new(&self.command);
        command
            .current_dir(&self.workspace)
            .args(&self.prefix_args)
            .arg(&prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let child = command.spawn().context("failed to spawn sidecar")?;
        let mut process_group = ProcessGroupGuard::new(child.id());

        let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
            Ok(output) => {
                process_group.disarm();
                output?
            }
            Err(_) => {
                return Err(anyhow!("sidecar timed out after {:?}", timeout));
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("sidecar failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            bail!("empty response");
        }

        Ok((stdout, truncated))
    }
}

struct ProcessGroupGuard {
    child_id: Option<u32>,
    armed: bool,
}

impl ProcessGroupGuard {
    fn new(child_id: Option<u32>) -> Self {
        Self { child_id, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        if self.armed {
            kill_process_group(self.child_id);
        }
    }
}

#[cfg(unix)]
fn kill_process_group(child_id: Option<u32>) {
    if let Some(child_id) = child_id {
        let process_group_id = -(child_id as libc::pid_t);
        // Best-effort cleanup for sidecars that spawn descendants; the timeout error is returned
        // regardless of whether the process already exited between timeout and this signal.
        unsafe {
            libc::kill(process_group_id, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill_process_group(_child_id: Option<u32>) {}

fn truncate_prompt(prompt: &str, max_chars: usize) -> (String, bool) {
    let truncated = prompt.chars().count() > max_chars;
    (prompt.chars().take(max_chars).collect(), truncated)
}

fn resolve_command(command: &str) -> Result<PathBuf> {
    let path = PathBuf::from(command);
    if path.exists() || path.is_absolute() || command.contains(std::path::MAIN_SEPARATOR) {
        return Ok(path);
    }
    which::which(command).with_context(|| format!("{command} command was not found in PATH"))
}

impl AiProvider {
    pub fn default_command(&self) -> Option<&'static str> {
        let (command, _) = self.invocation();
        (!command.is_empty()).then_some(command)
    }

    pub fn prefix_args(&self) -> &'static [&'static str] {
        let (_, prefix_args) = self.invocation();
        prefix_args
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use tempfile::TempDir;

    use super::*;

    fn write_script(dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, "#!/bin/sh\nprintf 'ok\\n'\n").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }

    #[test]
    fn provider_prefix_args_match_invocation() {
        for provider in
            [AiProvider::Claude, AiProvider::Codex, AiProvider::Gemini, AiProvider::Custom]
        {
            let (_, prefix_args) = provider.invocation();
            assert_eq!(provider.prefix_args(), prefix_args);
        }
    }

    #[test]
    fn new_uses_provider_specific_prefix_args() {
        let dir = TempDir::new().unwrap();
        let script = write_script(&dir, "mock-ai.sh");
        let config = AiConfig {
            enabled: true,
            provider: AiProvider::Codex,
            command: Some(script.display().to_string()),
            timeout_secs: 1,
        };

        let adapter = SidecarAdapter::new(dir.path(), &config).unwrap();
        assert_eq!(adapter.prefix_args, vec!["exec".to_string()]);
    }

    #[test]
    fn from_command_defaults_to_claude_prefix_args() {
        let dir = TempDir::new().unwrap();
        let script = write_script(&dir, "mock-ai.sh");

        let adapter =
            SidecarAdapter::from_command(dir.path(), script, Duration::from_secs(1)).unwrap();
        assert_eq!(adapter.prefix_args, vec!["-p".to_string()]);
    }
}
