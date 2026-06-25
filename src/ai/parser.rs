use crate::message::{AiIntent, AiPayload, StructuredOutput};

pub fn parse_ai_payload(raw: &str) -> AiPayload {
    let mut intent = None;
    let mut text = None;
    let mut structured = None;
    let mut text_buffer: Option<String> = None;

    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("INTENT:") {
            if let Some(value) = text_buffer.take() {
                text = Some(value.trim().to_string());
            }
            intent = Some(parse_intent(value.trim()));
        } else if let Some(value) = line.strip_prefix("TEXT:") {
            if let Some(value) = text_buffer.take() {
                text = Some(value.trim().to_string());
            }
            text_buffer = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("STRUCTURED:") {
            if let Some(value) = text_buffer.take() {
                text = Some(value.trim().to_string());
            }
            structured = parse_structured_output(value.trim());
        } else if let Some(value) = text_buffer.as_mut() {
            if !value.is_empty() {
                value.push('\n');
            }
            value.push_str(line);
        }
    }
    if let Some(value) = text_buffer {
        text = Some(value.trim().to_string());
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

fn parse_structured_output(value: &str) -> Option<StructuredOutput> {
    let mut output = serde_json::from_str::<StructuredOutput>(value).ok()?;
    if !output.validate() {
        return None;
    }
    output.sanitize_skill_suggestions();
    Some(output)
}
