use std::fs;
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use triadchat::application::Application;
use triadchat::config::Config;

#[test]
fn art_shortcode_is_expanded_before_sending() {
    let dir = TempDir::new().unwrap();
    let script = dir.path().join("mock-claude.sh");
    fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let workspace = TempDir::new().unwrap();
    let art_yaml = workspace.path().join("art.yaml");
    fs::write(&art_yaml, "smile: |\n  (^_^)\n  </  /\n  <) )>\n").unwrap();

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;

    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("look: [smile]").unwrap();

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains("(^_^)"), "expected expanded smile art, got: {rendered}");
    assert!(!rendered.contains("[smile]"), "shortcode should be replaced, got: {rendered}");
}

#[test]
fn art_shortcode_not_in_dict_passes_through() {
    let dir = TempDir::new().unwrap();
    let script = dir.path().join("mock-claude.sh");
    fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let workspace = TempDir::new().unwrap();
    let art_yaml = workspace.path().join("art.yaml");
    fs::write(&art_yaml, "logo: TRIADCHAT\n").unwrap();

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;

    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("[nope]").unwrap();

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains("[nope]"), "unknown shortcode should pass through, got: {rendered}");
}

#[test]
fn art_list_shows_shorhand_codes() {
    let dir = TempDir::new().unwrap();
    let script = dir.path().join("mock-claude.sh");
    fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let workspace = TempDir::new().unwrap();
    let art_yaml = workspace.path().join("art.yaml");
    fs::write(&art_yaml, "smile: (^_^)\nlogo: TRIADCHAT\nwave: o/\n").unwrap();

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;

    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("/art list").unwrap();

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains("Art shortcodes:"));
    assert!(rendered.contains("logo"));
    assert!(rendered.contains("smile"));
    assert!(rendered.contains("wave"));
}

#[test]
fn art_reload_replaces_dictionary() {
    let dir = TempDir::new().unwrap();
    let script = dir.path().join("mock-claude.sh");
    fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let workspace = TempDir::new().unwrap();
    let art_yaml = workspace.path().join("art.yaml");
    fs::write(&art_yaml, "old: old-art\n").unwrap();

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;

    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("[old]").unwrap();

    fs::write(&art_yaml, "new: new-art\n").unwrap();
    app.handle_input_line_for_test("/art reload").unwrap();

    app.handle_input_line_for_test("[new]").unwrap();

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains("old-art"));
    assert!(rendered.contains("new-art"));
    assert!(rendered.contains("Art dictionary reloaded (1 entries)"));
}

#[test]
fn art_shortcode_with_unicode_key_and_trailing_text() {
    let dir = TempDir::new().unwrap();
    let script = dir.path().join("mock-claude.sh");
    fs::write(&script, "#!/bin/sh\nprintf 'ok'\n").unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();

    let workspace = TempDir::new().unwrap();
    let art_yaml = workspace.path().join("art.yaml");
    fs::write(&art_yaml, "猫: Neko\n").unwrap();

    let mut config = Config::default();
    config.ai.command = Some(script.display().to_string());
    config.user_name = "tester".into();
    config.terminal_bell = false;

    let mut app = Application::new_for_test_in_workspace(&config, workspace.path()).unwrap();

    app.handle_input_line_for_test("[猫] trailing").unwrap();

    let rendered =
        app.state().messages().iter().map(|m| m.rendered_text()).collect::<Vec<_>>().join("\n");

    assert!(rendered.contains("Neko"), "expected Neko, got: {rendered}");
    assert!(rendered.contains("trailing"), "expected trailing, got: {rendered}");
    assert!(!rendered.contains("[猫]"), "shortcode should be replaced, got: {rendered}");
}
