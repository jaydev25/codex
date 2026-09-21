use super::*;
use pretty_assertions::assert_eq;

fn assignment() -> ActorAssignment {
    ActorAssignment {
        schema_version: 1,
        task_id: "task-1".to_string(),
        kind: ActorTaskKind::UnitTest,
        objective: "Add tests for the parser".to_string(),
        allowed_paths: vec!["src/parser.rs".to_string()],
        allowed_tools: vec!["apply_patch".to_string()],
        tool_call_map: vec![ActorToolCallMap {
            operation: "write_tests".to_string(),
            tool: "apply_patch".to_string(),
            argument_template: serde_json::json!({ "path": "src/parser.rs" }),
        }],
        acceptance_criteria: vec!["parser tests pass".to_string()],
    }
}

#[test]
fn planner_assignment_is_directly_embedded_in_actor_system_message() {
    let assignment = assignment();
    let body = actor_request_body(
        &Url::parse("http://127.0.0.1:1234/v1").unwrap(),
        "qwen/qwen3-coder-30b",
        &assignment,
        &[],
    )
    .unwrap();
    let system_content = body["messages"][0]["content"].as_str().unwrap();
    let assignment_json = serde_json::to_string(&assignment).unwrap();
    assert!(system_content.ends_with(&assignment_json));
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["model"], "qwen/qwen3-coder-30b");
}

#[test]
fn assignment_rejects_unapproved_tool_and_remote_endpoint() {
    let mut invalid_assignment = assignment();
    invalid_assignment.tool_call_map[0].tool = "exec_command".to_string();
    assert!(
        actor_request_body(
            &Url::parse("http://127.0.0.1:1234/v1").unwrap(),
            "qwen/qwen3-coder-30b",
            &invalid_assignment,
            &[],
        )
        .is_err()
    );
    assert!(
        actor_request_body(
            &Url::parse("https://example.com/v1").unwrap(),
            "qwen/qwen3-coder-30b",
            &assignment(),
            &[],
        )
        .is_err()
    );
}

#[test]
fn third_failed_actor_attempt_escalates_original_assignment() {
    let original = assignment();
    let mut tracker = ActorAttemptTracker::new(original.clone());
    assert_eq!(
        tracker.record_failure("compile failed"),
        ActorAttemptDecision::Retry { next_attempt: 2 }
    );
    assert_eq!(
        tracker.record_failure("unit test failed"),
        ActorAttemptDecision::Retry { next_attempt: 3 }
    );
    let expected = ActorAttemptDecision::Escalate(ActorEscalation {
        original_assignment: original,
        failures: vec![
            "compile failed".to_string(),
            "unit test failed".to_string(),
            "E2E test failed".to_string(),
        ],
    });
    assert_eq!(tracker.record_failure("E2E test failed"), expected);
    assert_eq!(tracker.record_failure("ignored fourth failure"), expected);
}
