pub fn lang_instruction(lang: &str) -> &'static str {
    match lang {
        "ja" => "必ず日本語で出力してください。",
        "en" => "Respond in English.",
        "zh" => "请用中文回答。",
        "ko" => "한국어로 답변해 주세요。",
        _ => "Respond in English.",
    }
}

pub fn truncate_transcript(transcript: &str, max_lines: usize) -> String {
    let lines = transcript.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn escape_xml(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn sanitize_untrusted_text(value: &str) -> String {
    value
        .lines()
        .map(|line| {
            let escaped = escape_xml(line);
            if is_control_line(&escaped) {
                format!("user-content: {escaped}")
            } else {
                escaped
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_control_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("TASK:")
        || trimmed.starts_with("INTENT:")
        || trimmed.starts_with("TEXT:")
        || trimmed.starts_with("STRUCTURED:")
        || trimmed.starts_with("TRANSCRIPT:")
        || trimmed.starts_with("LAST_MESSAGES:")
        || trimmed.starts_with("QUESTION:")
}

fn tagged(tag: &str, value: &str) -> String {
    format!("<{tag}>\n{}\n</{tag}>", sanitize_untrusted_text(value))
}

fn base_prompt(task: &str, transcript: &str, lang: &str) -> String {
    let transcript = tagged("transcript", &truncate_transcript(transcript, 100));
    format!(
        "TASK:{task}\n{}\n\
Return the answer in exactly this format:\n\
INTENT: <Clarify|Summary|Todo|Decision|SkillSuggest>\n\
TEXT: <summary text>\n\
STRUCTURED: {{\"todos\":[{{\"text\":\"...\",\"assignee\":\"...\"}}],\"decisions\":[\"...\"],\"skill_suggestions\":[\"...\"]}}\n\
{}\n",
        lang_instruction(lang),
        transcript
    )
}

pub fn summary_prompt(transcript: &str, lang: &str) -> String {
    base_prompt("summary", transcript, lang)
}

pub fn todos_prompt(transcript: &str, lang: &str) -> String {
    base_prompt("todos", transcript, lang)
}

pub fn decisions_prompt(transcript: &str, lang: &str) -> String {
    base_prompt("decisions", transcript, lang)
}

pub fn intervene_prompt(transcript: &str, last_messages: &[String], lang: &str) -> String {
    let messages = last_messages
        .iter()
        .map(|message| format!("<message>{}</message>", sanitize_untrusted_text(message)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n<last_messages>\n{}\n</last_messages>\n",
        base_prompt("intervene", transcript, lang),
        messages
    )
}

pub fn mention_prompt(message: &str, transcript: &str, lang: &str) -> String {
    let question = tagged("question", message);
    let context = tagged("recent_context", &truncate_transcript(transcript, 10));
    format!(
        "TASK:mention\n{}\n\
        You are ops-ai, a helpful team member who was directly addressed.\n\
        Rules:\n\
        - Answer the QUESTION below with actual content (1-3 sentences).\n\
        - Do NOT start your answer by describing or restating the question.\n\
        - Do NOT write 'The user is asking...' or 'ユーザーが〜と質問しています' or similar meta-commentary.\n\
        - Do NOT generate TODO items or decisions — always leave STRUCTURED arrays empty.\n\
        - Be direct and conversational, like a knowledgeable teammate.\n\
        Return EXACTLY this format (no other text):\n\
        INTENT: Clarify\n\
        TEXT: <your direct answer here>\n\
        STRUCTURED: {{\"todos\":[],\"decisions\":[],\"skill_suggestions\":[]}}\n\
        {}\n\
        {}\n",
        lang_instruction(lang),
        question,
        context
    )
}

pub fn companion_prompt(transcript: &str, last_messages: &[String], lang: &str) -> String {
    let messages = last_messages
        .iter()
        .map(|message| format!("<message>{}</message>", sanitize_untrusted_text(message)))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n\
        You are an active conversation participant, not just a clerk.\n\
        React naturally: add relevant ideas, ask clarifying questions,\n\
        point out interesting angles, or summarise when helpful.\n\
        Keep responses short (1-3 sentences). Do not summarise unless asked.\n\
        <last_messages>\n{}\n</last_messages>\n",
        base_prompt("companion", transcript, lang),
        messages
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mention_prompt_contains_question() {
        let prompt = mention_prompt("熱海はどのような場所？", "alice: 旅行の話", "ja");
        assert!(prompt.contains("熱海はどのような場所？"));
    }

    #[test]
    fn mention_prompt_contains_no_meta_commentary_instruction() {
        let prompt = mention_prompt("question", "transcript", "en");
        assert!(prompt.contains("Do NOT start your answer by describing or restating"));
        assert!(prompt.contains("Do NOT generate TODO items"));
    }

    #[test]
    fn mention_prompt_includes_fixed_intent_clarify() {
        let prompt = mention_prompt("question", "transcript", "en");
        assert!(prompt.contains("INTENT: Clarify"));
    }

    #[test]
    fn mention_prompt_includes_context_transcript() {
        let prompt = mention_prompt("question", "alice: hello\nbob: hi", "en");
        assert!(prompt.contains("alice: hello"));
    }

    #[test]
    fn mention_prompt_differs_from_intervene_prompt() {
        let mention = mention_prompt("question", "transcript", "en");
        let intervene = intervene_prompt("transcript", &["question".to_string()], "en");
        assert_ne!(mention, intervene);
    }

    #[test]
    fn mention_prompt_respects_lang() {
        let ja = mention_prompt("質問", "transcript", "ja");
        assert!(ja.contains("日本語"));
    }
}
