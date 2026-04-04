use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use tokio::process::Command;

use crate::config::AiConfig;

#[derive(Clone, Debug)]
pub struct SidecarAdapter {
    workspace: PathBuf,
    command: PathBuf,
    timeout: Duration,
}

impl SidecarAdapter {
    pub fn new(workspace: &Path, config: &AiConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_secs.max(1));
        let command = if let Some(command) = &config.command {
            resolve_command(command)?
        } else {
            which::which("claude").context("claude command was not found in PATH")?
        };
        Self::from_command(workspace, command, timeout)
    }

    pub fn from_command(
        workspace: &Path,
        command: impl Into<PathBuf>,
        timeout: Duration,
    ) -> Result<Self> {
        let command = command.into();
        if !command.exists() {
            bail!("sidecar command does not exist: {}", command.display());
        }
        Ok(Self { workspace: workspace.to_path_buf(), command, timeout })
    }

    pub async fn ask(&self, prompt: &str) -> Result<String> {
        self.run_prompt(prompt, self.timeout).await
    }

    pub async fn run_skill(&self, skill_name: &str, args: &[String]) -> Result<String> {
        let suffix = if args.is_empty() { String::new() } else { format!(" {}", args.join(" ")) };
        self.run_prompt(&format!("/{skill_name}{suffix}"), Duration::from_secs(60)).await
    }

    async fn run_prompt(&self, prompt: &str, timeout: Duration) -> Result<String> {
        let prompt = truncate_prompt(prompt, 50_000);
        let output = tokio::time::timeout(timeout, async {
            Command::new(&self.command)
                .current_dir(&self.workspace)
                .arg("-p")
                .arg(prompt)
                .output()
                .await
        })
        .await
        .map_err(|_| anyhow!("sidecar timed out after {:?}", timeout))??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("sidecar failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            bail!("empty response");
        }

        Ok(stdout)
    }
}

fn truncate_prompt(prompt: &str, max_chars: usize) -> String {
    prompt.chars().take(max_chars).collect()
}

fn resolve_command(command: &str) -> Result<PathBuf> {
    let path = PathBuf::from(command);
    if path.exists() || path.is_absolute() || command.contains(std::path::MAIN_SEPARATOR) {
        return Ok(path);
    }
    which::which(command).with_context(|| format!("{command} command was not found in PATH"))
}
