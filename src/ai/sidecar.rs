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
            PathBuf::from(command)
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
        let prompt = truncate_prompt(prompt, 50_000);
        let output = tokio::time::timeout(self.timeout, async {
            Command::new(&self.command)
                .current_dir(&self.workspace)
                .arg("-p")
                .arg(prompt)
                .output()
                .await
        })
        .await
        .map_err(|_| anyhow!("sidecar timed out after {:?}", self.timeout))??;

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
