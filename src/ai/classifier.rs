#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageClass {
    Discuss,
    Decide,
    Task,
    Execute,
}

impl MessageClass {
    pub fn classify(input: &str) -> Self {
        if contains_decision_marker(input) {
            Self::Decide
        } else if contains_task_marker(input) {
            Self::Task
        } else if contains_execute_request(input) {
            Self::Execute
        } else {
            Self::Discuss
        }
    }
}

pub fn contains_decision_marker(input: &str) -> bool {
    let normalized = input.to_lowercase();
    normalized.contains("決定")
        || normalized.contains("結論")
        || normalized.contains("decide")
        || normalized.contains("decided")
        || normalized.contains("決まり")
}

pub fn contains_task_marker(input: &str) -> bool {
    let normalized = input.to_lowercase();
    normalized.contains("todo")
        || normalized.contains("担当")
        || normalized.contains("書く")
        || normalized.contains("fix")
        || normalized.contains("対応")
        || normalized.contains("やる")
        || normalized.contains("task")
}

pub fn contains_execute_request(input: &str) -> bool {
    let normalized = input.to_lowercase();
    normalized.contains("/skill")
        || normalized.contains("実行")
        || normalized.contains("run")
        || normalized.contains("deploy")
        || normalized.contains("apply")
        || normalized.contains("してください")
        || normalized.contains("やって")
}

pub fn contains_ambiguity(input: &str) -> bool {
    let normalized = input.to_lowercase();
    normalized.contains('?')
        || normalized.contains('？')
        || normalized.contains("どう")
        || normalized.contains("どっち")
        || normalized.contains("迷")
        || normalized.contains("悩")
        || normalized.contains("which")
        || normalized.contains("should we")
        || normalized.contains("unclear")
        || normalized.contains("不明")
        || normalized.contains("曖昧")
}

pub fn contains_contradiction(input: &str) -> bool {
    let normalized = input.to_lowercase();
    normalized.contains("矛盾")
        || normalized.contains("contradict")
        || normalized.contains("一方で")
        || normalized.contains("but")
        || normalized.contains("しかし")
}
