use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillScope {
    User,
    #[default]
    Workspace,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvokeMode {
    Manual,
    #[default]
    Confirm,
    AutoSafe,
    Suggest,
}

impl InvokeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Confirm => "confirm",
            Self::AutoSafe => "auto-safe",
            Self::Suggest => "suggest",
        }
    }
}

impl fmt::Display for InvokeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    #[default]
    Medium,
    High,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub scope: SkillScope,
    pub invoke_mode: InvokeMode,
    pub allowed_tools: Vec<String>,
    pub risk: RiskLevel,
    pub description: String,
    pub args_hint: Option<String>,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct SkillRegistry {
    skills: Vec<SkillMeta>,
}

impl SkillRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn scan(workspace: &Path) -> Self {
        let Some(cache_base) = dirs_next::data_dir() else {
            return Self::scan_with_cache_base(workspace, workspace);
        };
        Self::scan_with_cache_base(workspace, cache_base)
    }

    pub fn scan_with_cache_base(workspace: &Path, cache_base: impl AsRef<Path>) -> Self {
        let skills_dir = workspace.join(".claude/skills");
        if !skills_dir.exists() {
            return Self::empty();
        }

        let cache_path = Self::cache_path(cache_base);
        let cache = read_cache(&cache_path);
        let mut skills = Vec::new();

        let Ok(entries) = fs::read_dir(&skills_dir) else {
            return Self::empty();
        };

        for entry in entries.flatten() {
            let skill_path = entry.path().join("SKILL.md");
            if !skill_path.exists() {
                continue;
            }

            let mtime = file_mtime_ms(&skill_path);
            if let Some(cached) = cache.get(skill_path.to_string_lossy().as_ref()) {
                if cached.mtime_ms == mtime {
                    skills.push(cached.meta.clone());
                    continue;
                }
            }

            let Ok(raw) = fs::read_to_string(&skill_path) else {
                continue;
            };
            let Some(frontmatter) = extract_frontmatter(&raw) else {
                tracing::warn!("skill frontmatter missing: {}", skill_path.display());
                continue;
            };
            let Ok(parsed) = serde_yaml::from_str::<Frontmatter>(&frontmatter) else {
                tracing::warn!("skill frontmatter parse failed: {}", skill_path.display());
                continue;
            };
            let description = if parsed.description.is_empty() {
                raw.lines()
                    .find(|line| !line.trim().is_empty() && !line.starts_with("---"))
                    .unwrap_or_default()
                    .trim_start_matches('#')
                    .trim()
                    .to_string()
            } else {
                parsed.description
            };

            skills.push(SkillMeta {
                name: parsed.name,
                scope: parsed.scope.unwrap_or_default(),
                invoke_mode: parsed.invoke.unwrap_or_default(),
                allowed_tools: parsed.allowed_tools.unwrap_or_default(),
                risk: parsed.risk.unwrap_or_default(),
                description,
                args_hint: parsed.args_hint.filter(|hint| !hint.trim().is_empty()),
                path: skill_path.clone(),
            });
        }

        skills.sort_by(|left, right| left.name.cmp(&right.name));
        write_cache(&cache_path, &skills);
        Self { skills }
    }

    pub fn skills(&self) -> &[SkillMeta] {
        &self.skills
    }

    pub fn find(&self, name: &str) -> Option<&SkillMeta> {
        self.skills.iter().find(|skill| skill.name == name)
    }

    pub fn cache_path(base: impl AsRef<Path>) -> PathBuf {
        base.as_ref().join("triadchat/skills_cache.json")
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Frontmatter {
    name: String,
    #[serde(default)]
    scope: Option<SkillScope>,
    #[serde(default, alias = "invoke")]
    invoke: Option<InvokeMode>,
    #[serde(default, rename = "allowed-tools")]
    allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    risk: Option<RiskLevel>,
    #[serde(default)]
    description: String,
    #[serde(default, rename = "args_hint", alias = "args-hint")]
    args_hint: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct SkillCache {
    entries: Vec<SkillCacheEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SkillCacheEntry {
    path: String,
    mtime_ms: u128,
    meta: SkillMeta,
}

fn extract_frontmatter(raw: &str) -> Option<String> {
    let mut parts = raw.splitn(3, "---");
    let _ = parts.next()?;
    let frontmatter = parts.next()?.trim();
    Some(frontmatter.to_string())
}

fn read_cache(path: &Path) -> HashMap<String, SkillCacheEntry> {
    let Ok(raw) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    let Ok(cache) = serde_json::from_str::<SkillCache>(&raw) else {
        return HashMap::new();
    };
    cache.entries.into_iter().map(|entry| (entry.path.clone(), entry)).collect()
}

fn write_cache(path: &Path, skills: &[SkillMeta]) {
    let entries = skills
        .iter()
        .map(|meta| SkillCacheEntry {
            path: meta.path.to_string_lossy().to_string(),
            mtime_ms: file_mtime_ms(&meta.path),
            meta: meta.clone(),
        })
        .collect::<Vec<_>>();
    let cache = SkillCache { entries };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(&cache) {
        let _ = fs::write(path, raw);
    }
}

fn file_mtime_ms(path: &Path) -> u128 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
