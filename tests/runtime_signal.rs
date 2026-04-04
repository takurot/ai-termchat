use std::time::Duration;

use triadchat::application::{Application, Signal};
use triadchat::config::Config;
use triadchat::message::{AiIntent, AiPayload, StructuredOutput};

#[test]
fn tokio_runtime_can_send_ai_signal_back_into_application_loop() {
    let config = Config::default();
    let mut app = Application::new_for_test(&config).expect("application should build");
    let node = app.node_handler();

    app.runtime_handle().spawn(async move {
        tokio::time::sleep(Duration::from_millis(25)).await;
        node.signals().send(Signal::AiResponse(AiPayload {
            text: "summary from runtime".into(),
            intent: AiIntent::Summary,
            structured: Some(StructuredOutput::default()),
        }));
    });

    app.process_next_event_for_test().expect("ai signal should be processed");

    let messages = app.state().messages();
    let ai_message = messages.last().expect("application should store the ai response");
    assert!(ai_message.rendered_text().contains("summary from runtime"));
}
