pub struct Messages {
    pub connected: &'static str,
    pub disconnected: &'static str,
    pub thinking_title: &'static str,
}

const JA_MESSAGES: Messages = Messages {
    connected: " が接続しました",
    disconnected: " が切断しました",
    thinking_title: "[ops-ai: 考え中...]",
};

const EN_MESSAGES: Messages = Messages {
    connected: " is online",
    disconnected: " is offline",
    thinking_title: "[ops-ai: thinking...]",
};

pub fn messages(lang: &str) -> &'static Messages {
    match lang {
        "ja" => &JA_MESSAGES,
        _ => &EN_MESSAGES,
    }
}
