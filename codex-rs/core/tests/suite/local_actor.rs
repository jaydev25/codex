//! Exercises the cloud-planner-to-local-actor tool boundary without executing proposals.

use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::test_codex::test_codex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde_json::json;
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_assignment_reaches_actor_system_prompt_and_returns_proposals() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let cloud = responses::start_mock_server().await;
    let actor = MockServer::start().await;
    let assignment = json!({
        "schema_version": 1,
        "task_id": "task-1",
        "kind": "unit_test",
        "objective": "Propose a parser unit test without changing files",
        "allowed_paths": ["src/parser.rs"],
        "allowed_tools": ["apply_patch"],
        "tool_call_map": [{
            "operation": "write_tests",
            "tool": "apply_patch",
            "argument_template": {"path": "src/parser.rs"}
        }],
        "acceptance_criteria": ["parser unit tests pass"]
    });
    let actor_result = json!({
        "schema_version": 1,
        "task_id": "task-1",
        "proposed_patches": [{
            "path": "src/parser.rs",
            "apply_patch": "*** Begin Patch\n*** Add File: src/parser.rs\n+#[test] fn parser_smoke() {}\n*** End Patch"
        }],
        "test_commands": ["cargo test parser_smoke"],
        "diagnostics": ["Patch proposal is not applied in this boundary test"],
        "needs_escalation": false
    });
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": actor_result.to_string()}}]
        })))
        .expect(1)
        .mount(&actor)
        .await;
    let call_id = "actor-call";
    let cloud_responses = responses::mount_sse_sequence(
        &cloud,
        vec![
            responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_function_call(call_id, "local_actor", &assignment.to_string()),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-2"),
                responses::ev_assistant_message("message", "done"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;
    let actor_url = format!("{}/v1", actor.uri());
    let mut builder = test_codex().with_config(move |config| {
        config.local_analysis.enabled = true;
        config.local_analysis.backend_url = Some(actor_url);
        config.local_analysis.backend_model = Some("qwen/qwen3-coder-30b".to_string());
    });
    let test = builder.build_with_auto_env(&cloud).await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Delegate a parser test assignment to the local actor.".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let actor_requests = actor.received_requests().await.unwrap_or_default();
    assert_eq!(actor_requests.len(), 1);
    let actor_body: serde_json::Value = actor_requests[0].body_json()?;
    let system = actor_body["messages"][0]["content"].as_str().unwrap();
    let sent_assignment: serde_json::Value = serde_json::from_str(
        system
            .split_once("Assignment JSON:\n")
            .expect("actor system prompt should contain the assignment")
            .1,
    )?;
    assert_eq!(sent_assignment, assignment);
    assert_eq!(actor_body["messages"][0]["role"], "system");
    assert!(actor_body["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("Prior failed attempts JSON:\n[]"));
    let cloud_requests = cloud_responses.requests();
    assert_eq!(cloud_requests.len(), 2);
    let planner_guidance = cloud_requests[0].message_input_texts("developer");
    assert!(planner_guidance.iter().any(|text| {
        text.contains("<local_actor_planner>")
            && text.contains("Send only structured JSON arguments through `local_actor`")
            && text.contains("After three unsuccessful local iterations")
    }));
    assert!(
        cloud_requests[1]
            .function_call_output(call_id)
            .to_string()
            .contains("Patch proposal is not applied in this boundary test")
    );
    assert!(cloud_requests[1]
        .function_call_output(call_id)
        .to_string()
        .contains("parser_smoke"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn third_failed_actor_attempt_returns_original_task_to_cloud() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let cloud = responses::start_mock_server().await;
    let actor = MockServer::start().await;
    let assignment = json!({
        "schema_version": 1,
        "task_id": "retry-task",
        "kind": "debug",
        "objective": "Fix the original parser failure",
        "allowed_paths": ["src/parser.rs"],
        "allowed_tools": ["apply_patch"],
        "tool_call_map": [{"operation": "repair", "tool": "apply_patch", "argument_template": {}}],
        "acceptance_criteria": ["parser tests pass"]
    });
    let actor_result = json!({
        "schema_version": 1,
        "task_id": "retry-task",
        "proposed_patches": [],
        "test_commands": [],
        "diagnostics": [],
        "needs_escalation": false
    });
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": actor_result.to_string()}}]
        })))
        .expect(3)
        .mount(&actor)
        .await;
    let mut second = assignment.clone();
    second["failure_feedback"] = json!("compile failed");
    let mut third = assignment.clone();
    third["failure_feedback"] = json!("unit test failed");
    let mut fourth = assignment.clone();
    fourth["failure_feedback"] = json!("E2E test failed");
    let calls = [assignment, second, third, fourth];
    let mut sequence = Vec::new();
    for (index, call) in calls.iter().enumerate() {
        let call_id = format!("actor-{index}");
        let response_id = format!("resp-{index}");
        sequence.push(responses::sse(vec![
            responses::ev_response_created(&response_id),
            responses::ev_function_call(&call_id, "local_actor", &call.to_string()),
            responses::ev_completed(&response_id),
        ]));
    }
    sequence.push(responses::sse(vec![
        responses::ev_response_created("final"),
        responses::ev_assistant_message("message", "done"),
        responses::ev_completed("final"),
    ]));
    let cloud_responses = responses::mount_sse_sequence(&cloud, sequence).await;
    let actor_url = format!("{}/v1", actor.uri());
    let test = test_codex()
        .with_config(move |config| {
            config.local_analysis.enabled = true;
            config.local_analysis.backend_url = Some(actor_url);
            config.local_analysis.backend_model = Some("qwen/qwen3-coder-30b".to_string());
        })
        .build_with_auto_env(&cloud)
        .await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Delegate and retry a local parser task.".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&test.codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let actor_requests = actor.received_requests().await.unwrap_or_default();
    assert_eq!(actor_requests.len(), 3);
    let retry_body: serde_json::Value = actor_requests[2].body_json()?;
    assert!(retry_body["messages"][0]["content"]
        .as_str()
        .unwrap()
        .contains("Prior failed attempts JSON:\n[\"compile failed\",\"unit test failed\"]"));
    let cloud_requests = cloud_responses.requests();
    assert_eq!(cloud_requests.len(), 5);
    let escalation = cloud_requests[4].function_call_output("actor-3");
    let escalation_text = escalation.to_string();
    assert!(escalation_text.contains("escalate"));
    assert!(escalation_text.contains("Fix the original parser failure"));
    assert!(escalation_text.contains("E2E test failed"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unavailable_local_actor_returns_original_task_to_cloud() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let cloud = responses::start_mock_server().await;
    let actor = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&actor)
        .await;
    let assignment = json!({
        "schema_version": 1,
        "task_id": "unavailable-task",
        "kind": "implementation",
        "objective": "Fix the original build failure",
        "allowed_paths": ["src/main.rs"],
        "allowed_tools": ["apply_patch"],
        "tool_call_map": [{"operation": "repair", "tool": "apply_patch", "argument_template": {}}],
        "acceptance_criteria": ["build passes"]
    });
    let cloud_responses = responses::mount_sse_sequence(
        &cloud,
        vec![
            responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_function_call("unavailable-call", "local_actor", &assignment.to_string()),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-2"),
                responses::ev_assistant_message("message", "done"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;
    let actor_url = format!("{}/v1", actor.uri());
    let test = test_codex()
        .with_config(move |config| {
            config.local_analysis.enabled = true;
            config.local_analysis.backend_url = Some(actor_url);
            config.local_analysis.backend_model = Some("qwen/qwen3-coder-30b".to_string());
        })
        .build_with_auto_env(&cloud)
        .await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Delegate a local implementation task.".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&test.codex, |event| matches!(event, EventMsg::TurnComplete(_))).await;

    let requests = cloud_responses.requests();
    let output = requests[1].function_call_output_text("unavailable-call").unwrap();
    let handoff: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(handoff["status"], "escalate");
    assert_eq!(handoff["original_assignment"], assignment);
    assert!(handoff["reason"].as_str().unwrap().contains("HTTP 503"));
    Ok(())
}
