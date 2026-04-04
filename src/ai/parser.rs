use crate::message::{AiIntent, AiPayload, StructuredOutput};

pub fn parse_ai_payload(raw: &str) -> AiPayload {
    let mut intent = None;
    let mut text = None;
    let mut structured = None;

    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("INTENT:") {
            intent = Some(parse_intent(value.trim()));
        } else if let Some(value) = line.strip_prefix("TEXT:") {
            text = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("STRUCTURED:") {
            structured = serde_json::from_str::<StructuredOutput>(value.trim()).ok();
        }
    }

    match (intent, text) {
        (Some(intent), Some(text)) => AiPayload { text, intent, structured },
        _ => AiPayload {
            text: raw.trim().to_string(),
            intent: AiIntent::Clarify,
            structured: Some(StructuredOutput::raw(raw)),
        },
    }
}

fn parse_intent(value: &str) -> AiIntent {
    match value {
        "Summary" => AiIntent::Summary,
        "Todo" => AiIntent::Todo,
        "Decision" => AiIntent::Decision,
        "SkillSuggest" => AiIntent::SkillSuggest,
        "Skip" => AiIntent::Skip,
        _ => AiIntent::Clarify,
    }
}
