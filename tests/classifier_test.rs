use triadchat::ai::classifier::MessageClass;

#[test]
fn classifier_detects_decision_language() {
    assert_eq!(MessageClass::classify("認証は service 層へ分けることで決定"), MessageClass::Decide);
}

#[test]
fn classifier_detects_task_language() {
    assert_eq!(MessageClass::classify("takuro が auth の設計を書く"), MessageClass::Task);
}

#[test]
fn classifier_defaults_to_discussion() {
    assert_eq!(MessageClass::classify("ここどう分けるのがよさそう？"), MessageClass::Discuss);
}
