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

fn base_prompt(task: &str, transcript: &str, lang: &str) -> String {
    format!(
        "TASK:{task}\n{}\n\
Return the answer in exactly this format:\n\
INTENT: <Clarify|Summary|Todo|Decision|SkillSuggest>\n\
TEXT: <summary text>\n\
STRUCTURED: {{\"todos\":[{{\"text\":\"...\",\"assignee\":\"...\"}}],\"decisions\":[\"...\"],\"skill_suggestions\":[\"...\"]}}\n\
TRANSCRIPT:\n{}\n",
        lang_instruction(lang),
        truncate_transcript(transcript, 100)
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
    format!(
        "{}\nLAST_MESSAGES:\n{}\n",
        base_prompt("intervene", transcript, lang),
        last_messages.join("\n")
    )
}

pub fn companion_prompt(transcript: &str, last_messages: &[String], lang: &str) -> String {
    format!(
        "{}\n\
        You are an active conversation participant, not just a clerk.\n\
        React naturally: add relevant ideas, ask clarifying questions,\n\
        point out interesting angles, or summarise when helpful.\n\
        Keep responses short (1-3 sentences). Do not summarise unless asked.\n\
        LAST_MESSAGES:\n{}\n",
        base_prompt("companion", transcript, lang),
        last_messages.join("\n")
    )
}
