//! Exercises the local-coordinator-to-cloud-worker boundary.

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
async fn local_model_coordinates_and_cloud_model_implements() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let cloud = responses::start_mock_server().await;
    let planner = MockServer::start().await;
    let planner_request = json!({
        "schema_version": 1,
        "phase": "plan",
        "objective": "Create a marker file and verify its contents",
        "constraints": ["Use the normal cloud tool path for every mutation"],
        "context_files": []
    });
    let planner_result = json!({
        "schema_version": 1,
        "summary": "Add the marker, then verify its observable contents.",
        "steps": [{
            "id": "write-marker",
            "owner": "cloud",
            "kind": "implement",
            "objective": "Create planner_test_marker.txt",
            "rationale": "The requested behavior is represented by one new file.",
            "files": ["planner_test_marker.txt"],
            "acceptance_criteria": ["The file contains planner-test-passed followed by a newline"],
            "suggested_command": null
        }, {
            "id": "verify-marker",
            "owner": "local",
            "kind": "verify",
            "objective": "Verify the marker contents",
            "rationale": "Verification is deterministic and repetitive.",
            "files": ["planner_test_marker.txt"],
            "acceptance_criteria": ["The command prints planner-test-passed"],
            "suggested_command": if cfg!(windows) { "Get-Content planner_test_marker.txt" } else { "cat planner_test_marker.txt" }
        }],
        "risks": ["A successful write alone does not verify file contents"],
        "verification": ["Read the file after writing it"],
        "needs_more_context": false,
        "requested_context_paths": []
    });
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"content": planner_result.to_string()}}],
            "usage": {
                "prompt_tokens": 600,
                "completion_tokens": 120,
                "total_tokens": 720
            }
        })))
        .expect(1)
        .mount(&planner)
        .await;

    let patch = "*** Begin Patch\n*** Add File: planner_test_marker.txt\n+planner-test-passed\n*** End Patch";
    let test_command = if cfg!(windows) {
        "Get-Content planner_test_marker.txt"
    } else {
        "cat planner_test_marker.txt"
    };
    let cloud_responses = responses::mount_sse_sequence(
        &cloud,
        vec![
            responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_function_call(
                    "planner-call",
                    "local_coordinator",
                    &planner_request.to_string(),
                ),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-2"),
                ev_apply_patch_custom_tool_call("cloud-patch", patch),
                responses::ev_completed("resp-2"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-3"),
                responses::ev_function_call(
                    "cloud-test",
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
    let planner_url = format!("{}/v1", planner.uri());
    let mut builder = test_codex().with_config(move |config| {
        config.local_analysis.enabled = true;
        config.local_analysis.backend_url = Some(planner_url);
        config.local_analysis.backend_model = Some("qwen-3.8-27b".to_string());
        config.permissions.approval_policy = Constrained::allow_any(AskForApproval::Never);
        config
            .permissions
            .set_permission_profile(PermissionProfile::Disabled)
            .expect("set planner integration test permissions");
    });
    let test = builder.build_with_auto_env(&cloud).await?;
    test.codex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "Create and verify a planner test marker.".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(&test.codex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let planner_requests = planner.received_requests().await.unwrap_or_default();
    assert_eq!(planner_requests.len(), 1);
    let planner_body: serde_json::Value = planner_requests[0].body_json()?;
    assert_eq!(planner_body["reasoning_budget_tokens"], 1_500);
    assert_eq!(planner_body["max_tokens"], 8_192);
    assert_eq!(planner_body["messages"][0]["role"], "system");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            planner_body["messages"][1]["content"].as_str().unwrap()
        )?,
        planner_request
    );

    let cloud_requests = cloud_responses.requests();
    let initial_body = cloud_requests[0].body_json();
    assert!(
        initial_body["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool["name"] == "local_coordinator")
    );
    assert!(
        cloud_requests[0]
            .message_input_texts("developer")
            .iter()
            .any(|text| {
                text.contains("<local_coordinator_guidance>")
                    && text.contains("you, the cloud worker")
            })
    );
    let tool_output: serde_json::Value = serde_json::from_str(
        &cloud_requests[1]
            .function_call_output_text("planner-call")
            .expect("local_coordinator should return function output"),
    )?;
    assert_eq!(tool_output["status"], "coordinated");
    assert_eq!(
        tool_output["next_action"],
        "execute_local_steps_or_delegate_cloud_owned_code_work"
    );
    assert_eq!(
        tokio::fs::read_to_string(test.cwd.path().join("planner_test_marker.txt")).await?,
        "planner-test-passed\n"
    );
    assert!(
        cloud_requests[3]
            .function_call_output_text("cloud-test")
            .expect("exec_command should return function output")
            .contains("planner-test-passed")
    );
    Ok(())
}
