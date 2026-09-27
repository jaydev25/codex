use super::*;

fn history_snapshot(rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>) -> String {
    drain_insert_history(rx)
        .iter()
        .map(|lines| lines_to_single_string(lines))
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn actor_model_context_input_snapshot() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.show_actor_context_prompt(
        "qwen3-coder-30b-a3b-instruct".to_string(),
        "qwen3-coder-30b-a3b-instruct".to_string(),
        Some(262_144),
    );

    assert_chatwidget_snapshot!(
        "actor_model_context_input",
        render_bottom_popup(&chat, /*width*/ 80)
    );
}

#[tokio::test]
async fn actor_model_loading_blocks_prompt_input_snapshot() {
    let (mut chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.begin_actor_model_activity(
        "Loading local actor model".to_string(),
        "qwen3-coder-30b-a3b-instruct · 32768 token context".to_string(),
    );

    assert_chatwidget_snapshot!(
        "actor_model_loading_blocks_prompt_input",
        normalize_snapshot_paths(render_bottom_popup(&chat, /*width*/ 80))
    );
}

#[tokio::test]
async fn actor_model_success_transition_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.set_actor_model("qwen3-coder-30b-a3b-instruct".to_string(), 32_768);

    assert_chatwidget_snapshot!("actor_model_success_transition", history_snapshot(&mut rx));
}

#[tokio::test]
async fn actor_model_invalid_context_retries_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let error = parse_actor_context_length("not-a-number", Some(262_144))
        .expect_err("invalid context length");

    chat.add_error_message(error);
    chat.show_actor_context_prompt(
        "qwen3-coder-30b-a3b-instruct".to_string(),
        "qwen3-coder-30b-a3b-instruct".to_string(),
        Some(262_144),
    );

    assert_chatwidget_snapshot!(
        "actor_model_invalid_context_retries",
        format!(
            "{}\n--- retry prompt ---\n{}",
            history_snapshot(&mut rx),
            render_bottom_popup(&chat, /*width*/ 80)
        )
    );
}

#[tokio::test]
async fn actor_model_load_failure_actions_snapshot() {
    let (mut chat, mut rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;

    chat.show_actor_model_load_failure(
        "qwen3-coder-30b-a3b-instruct".to_string(),
        "qwen3-coder-30b-a3b-instruct".to_string(),
        Some(262_144),
        32_768,
        "insufficient GPU memory".to_string(),
    );

    assert_chatwidget_snapshot!(
        "actor_model_load_failure_actions",
        format!(
            "{}\n--- recovery actions ---\n{}",
            history_snapshot(&mut rx),
            render_bottom_popup(&chat, /*width*/ 80)
        )
    );
}
