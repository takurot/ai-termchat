use triadchat::ai::parser::parse_ai_payload;
use triadchat::ai::prompt::{lang_instruction, summary_prompt, todos_prompt, truncate_transcript};
use triadchat::application::render_ai_payload;
use triadchat::message::{AiIntent, AiPayload, StructuredOutput, TodoItem};

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
fn render_todo_with_assignee() {
    let payload = AiPayload {
        text: "x".into(),
        intent: AiIntent::Todo,
        structured: Some(StructuredOutput {
            todos: vec![TodoItem { text: "foo".into(), assignee: Some("bar".into()) }],
            ..StructuredOutput::default()
        }),
    };
    assert_eq!(render_ai_payload(&payload), "TODO: foo (bar)");
}

#[test]
fn render_todo_without_assignee() {
    let payload = AiPayload {
        text: "x".into(),
        intent: AiIntent::Todo,
        structured: Some(StructuredOutput {
            todos: vec![TodoItem { text: "foo".into(), assignee: None }],
            ..StructuredOutput::default()
        }),
    };
    assert_eq!(render_ai_payload(&payload), "TODO: foo");
}

#[test]
fn render_decision() {
    let payload = AiPayload {
        text: "x".into(),
        intent: AiIntent::Decision,
        structured: Some(StructuredOutput {
            decisions: vec!["auth".into()],
            ..StructuredOutput::default()
        }),
    };
    assert_eq!(render_ai_payload(&payload), "Decision: auth");
}

#[test]
fn render_falls_back_to_text_for_empty_todos() {
    let payload = AiPayload {
        text: "fallback text".into(),
        intent: AiIntent::Todo,
        structured: Some(StructuredOutput::default()),
    };
    assert_eq!(render_ai_payload(&payload), "fallback text");
}

#[test]
fn render_returns_text_when_structured_is_none() {
    let payload =
        AiPayload { text: "raw text".into(), intent: AiIntent::Clarify, structured: None };
    assert_eq!(render_ai_payload(&payload), "raw text");
}
