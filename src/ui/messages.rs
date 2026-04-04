pub struct Messages {
    pub connected: &'static str,
    pub disconnected: &'static str,
    pub thinking_title: &'static str,
    pub acting_title: &'static str,
    pub failed_title: &'static str,
}

const JA_MESSAGES: Messages = Messages {
    connected: " が接続しました",
    disconnected: " が切断しました",
    thinking_title: "[ops-ai: 考え中...]",
    acting_title: "[ops-ai: 実行中...]",
    failed_title: "[ops-ai: 失敗]",
};

const EN_MESSAGES: Messages = Messages {
    connected: " is online",
    disconnected: " is offline",
    thinking_title: "[ops-ai: thinking...]",
    acting_title: "[ops-ai: acting...]",
    failed_title: "[ops-ai: failed]",
};

pub fn messages(lang: &str) -> &'static Messages {
    match lang {
        "ja" => &JA_MESSAGES,
        _ => &EN_MESSAGES,
    }
}
