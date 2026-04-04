use triadchat::ai::parser::parse_ai_payload;
use triadchat::ai::prompt::{lang_instruction, summary_prompt, todos_prompt, truncate_transcript};
use triadchat::message::AiIntent;

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
