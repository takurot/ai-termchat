#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageClass {
    Discuss,
    Decide,
    Task,
    Execute,
}

impl MessageClass {
    pub fn classify(input: &str) -> Self {
        let normalized = input.to_lowercase();
        if normalized.contains("決定")
            || normalized.contains("結論")
            || normalized.contains("decide")
        {
            Self::Decide
        } else if normalized.contains("todo")
            || normalized.contains("担当")
            || normalized.contains("書く")
            || normalized.contains("fix")
        {
            Self::Task
        } else if normalized.contains("/skill")
            || normalized.contains("実行")
            || normalized.contains("run")
        {
            Self::Execute
        } else {
            Self::Discuss
        }
    }
}

pub fn classify_message(input: &str) -> MessageClass {
    MessageClass::classify(input)
}
