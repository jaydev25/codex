//! Exercises the cloud-planner-to-local-actor boundary and reviewed proposal execution.

use anyhow::Result;
use codex_core::TurnInputRequest;
use codex_core::config::Constrained;
use codex_protocol::models::PermissionProfile;
use codex_protocol::protocol::AskForApproval;
use codex_protocol::protocol::EventMsg;
use codex_protocol::user_input::UserInput;
use core_test_support::responses;
use core_test_support::responses::ev_apply_patch_custom_tool_call;
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
async fn reviewed_actor_proposals_execute_through_normal_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let cloud = responses::start_mock_server().await;
    let actor = MockServer::start().await;
    let patch =
        "*** Begin Patch\n*** Add File: actor_test_marker.txt\n+actor-test-passed\n*** End Patch";
    let test_command = if cfg!(windows) {
        "Get-Content actor_test_marker.txt"
    } else {
        "cat actor_test_marker.txt"
    };
    let assignment = json!({
        "schema_version": 1,
        "task_id": "task-1",
        "kind": "unit_test",
        "objective": "Propose and validate an actor test marker",
        "allowed_paths": ["actor_test_marker.txt"],
        "allowed_tools": ["apply_patch", "exec_command"],
        "tool_call_map": [
            {
                "operation": "write_marker",
                "tool": "apply_patch",
                "argument_template": {"path": "actor_test_marker.txt"}
            },
            {
                "operation": "run_tests",
                "tool": "exec_command",
                "argument_template": {"cmd": "<actor-proposed test command>"}
            }
        ],
        "acceptance_criteria": ["actor test marker is readable"],
        "failure_feedback": null
    });
    let actor_result = json!({
        "schema_version": 1,
        "task_id": "task-1",
        "proposed_patches": [{
            "path": "actor_test_marker.txt",
            "apply_patch": patch
        }],
        "test_commands": [test_command],
        "diagnostics": ["Patch and test require cloud review before execution"],
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
                ev_apply_patch_custom_tool_call("actor-patch", patch),
                responses::ev_completed("resp-2"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-3"),
                responses::ev_function_call(
                    "actor-test",
                    "exec_command",
                    &json!({ "cmd": test_command }).to_string(),
                ),
                responses::ev_completed("resp-3"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-4"),
                responses::ev_assistant_message("message", "done"),
                responses::ev_completed("resp-4"),
            ]),
        ],
    )
    .await;
    let actor_url = format!("{}/v1", actor.uri());
    let mut builder = test_codex().with_config(move |config| {
        config.local_analysis.enabled = true;
        config.local_analysis.backend_url = Some(actor_url);
        config.local_analysis.backend_model = Some("qwen/qwen3-coder-30b".to_string());
        config.permissions.approval_policy = Constrained::allow_any(AskForApproval::Never);
        config
            .permissions
            .set_permission_profile(PermissionProfile::Disabled)
            .expect("set actor execution test permissions");
    });
    let test = builder.build_with_auto_env(&cloud).await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Delegate an actor test marker assignment to the local actor.".into(),
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
    let mut expected_assignment = assignment.clone();
    expected_assignment
        .as_object_mut()
        .expect("assignment should be an object")
        .remove("failure_feedback");
    assert_eq!(sent_assignment, expected_assignment);
    assert_eq!(actor_body["messages"][0]["role"], "system");
    assert!(
        actor_body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Prior failed attempts JSON:\n[]")
    );
    let cloud_requests = cloud_responses.requests();
    assert_eq!(cloud_requests.len(), 4);
    let request_body = cloud_requests[0].body_json();
    let actor_tool = request_body["tools"]
        .as_array()
        .expect("cloud request should contain tools")
        .iter()
        .find(|tool| tool["name"] == "local_actor")
        .expect("cloud request should contain local_actor");
    assert_eq!(actor_tool["strict"], true);
    let properties = actor_tool["parameters"]["properties"]
        .as_object()
        .expect("local_actor should define parameter properties");
    let required = actor_tool["parameters"]["required"]
        .as_array()
        .expect("local_actor should require every parameter");
    assert_eq!(required.len(), properties.len());
    assert_eq!(
        actor_tool["parameters"]["properties"]["failure_feedback"]["anyOf"],
        json!([{ "type": "string" }, { "type": "null" }])
    );
    let planner_guidance = cloud_requests[0].message_input_texts("developer");
    assert!(planner_guidance.iter().any(|text| {
        text.contains("<local_actor_planner>")
            && text.contains("especially lint fixes, isolated repairs")
            && text.contains("Send only structured JSON arguments through `local_actor`")
            && text.contains("author implementation and unit/E2E test patches")
            && text.contains("After three unsuccessful local iterations")
            && text.contains("hand that assignment back to `local_actor`")
            && text.contains("do not switch to cloud-authored implementation")
    }));
    assert!(
        cloud_requests[1]
            .function_call_output(call_id)
            .to_string()
            .contains("Patch and test require cloud review before execution")
    );
    let tool_output: serde_json::Value = serde_json::from_str(
        &cloud_requests[1]
            .function_call_output_text(call_id)
            .expect("local_actor should return function output"),
    )?;
    assert_eq!(
        tool_output["execution_plan"],
        json!([
            {
                "tool": "apply_patch",
                "arguments": patch,
                "reviewed": false
            },
            {
                "tool": "exec_command",
                "arguments": { "cmd": test_command },
                "reviewed": false
            }
        ])
    );
    assert_eq!(
        tool_output["acceptance_criteria"],
        json!(["actor test marker is readable"])
    );
    assert_eq!(
        tokio::fs::read_to_string(test.cwd.path().join("actor_test_marker.txt")).await?,
        "actor-test-passed\n"
    );
    assert!(
        cloud_requests[3]
            .function_call_output_text("actor-test")
            .expect("exec_command should return function output")
            .contains("actor-test-passed")
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn third_failed_actor_attempt_survives_resume_and_returns_task_to_cloud() -> Result<()> {
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
        "acceptance_criteria": ["parser tests pass"],
        "failure_feedback": null
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
    let sequence = vec![
        responses::sse(vec![
            responses::ev_response_created("resp-0"),
            responses::ev_function_call("actor-0", "local_actor", &assignment.to_string()),
            responses::ev_completed("resp-0"),
        ]),
        responses::sse(vec![
            responses::ev_response_created("resp-1"),
            responses::ev_function_call("actor-1", "local_actor", &second.to_string()),
            responses::ev_completed("resp-1"),
        ]),
        responses::sse(vec![
            responses::ev_response_created("pause"),
            responses::ev_assistant_message("pause-message", "retry after restart"),
            responses::ev_completed("pause"),
        ]),
        responses::sse(vec![
            responses::ev_response_created("resp-2"),
            responses::ev_function_call("actor-2", "local_actor", &third.to_string()),
            responses::ev_completed("resp-2"),
        ]),
        responses::sse(vec![
            responses::ev_response_created("resp-3"),
            responses::ev_function_call("actor-3", "local_actor", &fourth.to_string()),
            responses::ev_completed("resp-3"),
        ]),
        responses::sse(vec![
            responses::ev_response_created("final"),
            responses::ev_assistant_message("message", "done"),
            responses::ev_completed("final"),
        ]),
    ];
    let cloud_responses = responses::mount_sse_sequence(&cloud, sequence).await;
    let actor_url = format!("{}/v1", actor.uri());
    let resumed_actor_url = actor_url.clone();
    let mut initial_builder = test_codex().with_config(move |config| {
        config.local_analysis.enabled = true;
        config.local_analysis.backend_url = Some(actor_url);
        config.local_analysis.backend_model = Some("qwen/qwen3-coder-30b".to_string());
    });
    let initial = initial_builder.build_with_auto_env(&cloud).await?;
    initial
        .codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Delegate and retry a local parser task.".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&initial.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    let mut resume_builder = test_codex().with_config(move |config| {
        config.local_analysis.enabled = true;
        config.local_analysis.backend_url = Some(resumed_actor_url);
        config.local_analysis.backend_model = Some("qwen/qwen3-coder-30b".to_string());
    });
    let resumed = resume_builder.restart(&cloud, &initial).await?;
    resumed
        .codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Continue the same local parser task after restart.".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&resumed.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let actor_requests = actor.received_requests().await.unwrap_or_default();
    assert_eq!(actor_requests.len(), 3);
    let retry_body: serde_json::Value = actor_requests[2].body_json()?;
    assert!(
        retry_body["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Prior failed attempts JSON:\n[\"compile failed\",\"unit test failed\"]")
    );
    let cloud_requests = cloud_responses.requests();
    assert_eq!(cloud_requests.len(), 6);
    let escalation = cloud_requests[5].function_call_output("actor-3");
    let escalation_text = escalation.to_string();
    assert!(escalation_text.contains("escalate"));
    assert!(escalation_text.contains("cloud_replan_then_local_actor"));
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
                responses::ev_function_call(
                    "unavailable-call",
                    "local_actor",
                    &assignment.to_string(),
                ),
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
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = cloud_responses.requests();
    let output = requests[1]
        .function_call_output_text("unavailable-call")
        .unwrap();
    let handoff: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(handoff["status"], "escalate");
    assert_eq!(handoff["original_assignment"], assignment);
    assert!(handoff["reason"].as_str().unwrap().contains("HTTP 503"));
    Ok(())
}
