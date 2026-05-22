use triadchat::ai::parser::parse_ai_payload;
use triadchat::ai::prompt::{lang_instruction, summary_prompt, todos_prompt, truncate_transcript};
use triadchat::application::{Application, Signal};
use triadchat::config::Config;
use triadchat::message::{AiIntent, AiPayload, StructuredOutput};

#[test]
fn language_instruction_supports_supported_languages() {
    assert_eq!(lang_instruction("ja"), "必ず日本語で出力してください。");
    assert_eq!(lang_instruction("en"), "Respond in English.");
    assert_eq!(lang_instruction("zh"), "请用中文回答。");
    assert_eq!(lang_instruction("ko"), "한국어로 답변해 주세요。");
    assert_eq!(lang_instruction("fr"), "Respond in English.");
}

#[test]
fn truncate_transcript_keeps_latest_100_lines() {
    let input = (0..150).map(|index| format!("line-{index}")).collect::<Vec<_>>().join("\n");
    let truncated = truncate_transcript(&input, 100);

    assert!(!truncated.contains("line-0"));
    assert!(truncated.contains("line-50"));
    assert!(truncated.contains("line-149"));
}

#[test]
fn prompts_embed_format_contract() {
    let transcript = "takuro: auth serviceに切り出す\n";
    assert!(summary_prompt(transcript, "ja").contains("INTENT:"));
    assert!(todos_prompt(transcript, "ja").contains("STRUCTURED:"));
}

#[test]
fn parser_extracts_structured_output() {
    let raw = "INTENT: Todo\nTEXT: TODOを抽出しました\nSTRUCTURED: {\"todos\":[{\"text\":\"auth を分離\",\"assignee\":\"takuro\"}],\"decisions\":[\"auth は service に移す\"],\"skill_suggestions\":[\"review-auth\"]}\n";
    let payload = parse_ai_payload(raw);

    assert_eq!(payload.intent, AiIntent::Todo);
    assert_eq!(payload.structured.as_ref().unwrap().todos.len(), 1);
    assert_eq!(payload.structured.as_ref().unwrap().todos[0].assignee.as_deref(), Some("takuro"));
}

#[test]
fn parser_collects_multiline_text_until_next_prefix() {
    let raw = "INTENT: Summary\nTEXT: First line\nSecond line\nThird line\nSTRUCTURED: {\"todos\":[],\"decisions\":[\"ship it\"],\"skill_suggestions\":[]}\n";
    let payload = parse_ai_payload(raw);

    assert_eq!(payload.intent, AiIntent::Summary);
    assert_eq!(payload.text, "First line\nSecond line\nThird line");
    assert_eq!(payload.structured.as_ref().unwrap().decisions, vec!["ship it".to_string()]);
}

#[test]
fn parser_collects_multiline_text_until_end_of_string() {
    let raw = "INTENT: Summary\nTEXT: First line\nSecond line\nThird line";
    let payload = parse_ai_payload(raw);

    assert_eq!(payload.intent, AiIntent::Summary);
    assert_eq!(payload.text, "First line\nSecond line\nThird line");
}

#[test]
fn parser_falls_back_without_panicking() {
    let payload = parse_ai_payload("unexpected raw response");
    assert_eq!(payload.intent, AiIntent::Clarify);
    assert!(payload.text.contains("unexpected raw response"));
}

#[test]
fn parser_malformed_structured_json_becomes_none() {
    let raw = "INTENT: Todo\nTEXT: some text\nSTRUCTURED: {not valid json}\n";
    let payload = parse_ai_payload(raw);
    assert_eq!(payload.intent, AiIntent::Todo);
    assert_eq!(payload.text, "some text");
    assert!(payload.structured.is_none());
}

#[test]
fn parser_skill_suggest_intent() {
    let raw = "INTENT: SkillSuggest\nTEXT: some text\n";
    let payload = parse_ai_payload(raw);
    assert_eq!(payload.intent, AiIntent::SkillSuggest);
}

#[test]
fn parser_skip_intent() {
    let raw = "INTENT: Skip\nTEXT: some text\n";
    let payload = parse_ai_payload(raw);
    assert_eq!(payload.intent, AiIntent::Skip);
}

#[test]
fn parser_unknown_intent_falls_back_to_clarify() {
    let raw = "INTENT: UnknownThing\nTEXT: some text\n";
    let payload = parse_ai_payload(raw);
    assert_eq!(payload.intent, AiIntent::Clarify);
}

#[test]
fn parser_multiple_metadata_lines_uses_last_value() {
    let raw = "INTENT: Todo\nINTENT: Summary\nTEXT: foo\nTEXT: bar\nSTRUCTURED: {\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}\nSTRUCTURED: {\"todos\":[],\"decisions\":[\"x\"],\"skill_suggestions\":[]}\n";
    let payload = parse_ai_payload(raw);
    assert_eq!(payload.intent, AiIntent::Summary);
    assert_eq!(payload.text, "bar");
    assert!(payload.structured.is_some());
    if let Some(structured) = &payload.structured {
        assert_eq!(structured.decisions, vec!["x"]);
    }
}

#[test]
fn structured_none_clears_skill_proposals() {
    let mut config = Config::default();
    config.ai.enabled = false;
    let mut app = Application::new_for_test(&config).unwrap();
    let node = app.node_handler();

    node.signals().send(Signal::AiResponse(
        AiPayload {
            text: "suggested".into(),
            intent: AiIntent::SkillSuggest,
            structured: Some(StructuredOutput {
                todos: Vec::new(),
                decisions: Vec::new(),
                skill_suggestions: vec!["review-auth".into()],
                raw_text: None,
            }),
        },
        false,
    ));
    app.process_next_event_for_test().unwrap();
    assert!(!app.state().skill_proposals().is_empty(), "should have proposals");

    node.signals().send(Signal::AiResponse(
        AiPayload { text: "raw text".into(), intent: AiIntent::Clarify, structured: None },
        false,
    ));
    app.process_next_event_for_test().unwrap();
    assert!(app.state().skill_proposals().is_empty(), "proposals should be cleared");
}
