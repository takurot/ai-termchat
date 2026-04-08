use tempfile::TempDir;

use triadchat::skill::registry::{InvokeMode, RiskLevel, SkillRegistry};

fn fixture_workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    let skills_dir = dir.path().join(".claude/skills");
    std::fs::create_dir_all(skills_dir.join("review-auth")).unwrap();
    std::fs::create_dir_all(skills_dir.join("broken-skill")).unwrap();

    std::fs::write(
        skills_dir.join("review-auth/SKILL.md"),
        r#"---
name: review-auth
invoke: confirm
risk: medium
allowed-tools: [Read, Grep]
description: 認証ロジックをレビューする
args_hint: <ticket-id>
---

# Review Auth
"#,
    )
    .unwrap();
    std::fs::write(
        skills_dir.join("broken-skill/SKILL.md"),
        r#"---
name: broken-skill
invoke: [not valid
---
"#,
    )
    .unwrap();

    dir
}

#[test]
fn scan_reads_valid_skills_and_skips_invalid_frontmatter() {
    let workspace = fixture_workspace();
    let cache_base = TempDir::new().unwrap();

    let registry = SkillRegistry::scan_with_cache_base(workspace.path(), cache_base.path());

    assert_eq!(registry.skills().len(), 1);
    let skill = registry.find("review-auth").expect("skill should be found");
    assert_eq!(skill.invoke_mode, InvokeMode::Confirm);
    assert_eq!(skill.risk, RiskLevel::Medium);
    assert_eq!(skill.allowed_tools, vec!["Read".to_string(), "Grep".to_string()]);
    assert_eq!(skill.args_hint.as_deref(), Some("<ticket-id>"));
}

#[test]
fn scan_uses_cache_file_without_breaking_results() {
    let workspace = fixture_workspace();
    let cache_base = TempDir::new().unwrap();

    let first = SkillRegistry::scan_with_cache_base(workspace.path(), cache_base.path());
    let second = SkillRegistry::scan_with_cache_base(workspace.path(), cache_base.path());

    assert_eq!(first.skills().len(), second.skills().len());
    assert!(SkillRegistry::cache_path(cache_base.path()).exists());
}

#[test]
fn missing_skills_directory_returns_empty_registry() {
    let workspace = TempDir::new().unwrap();

    let registry = SkillRegistry::scan(workspace.path());

    assert!(registry.skills().is_empty());
}

#[test]
fn display_labels_are_human_readable() {
    assert_eq!(InvokeMode::AutoSafe.to_string(), "auto-safe");
    assert_eq!(RiskLevel::Low.to_string(), "low");
}
