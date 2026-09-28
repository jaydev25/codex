use super::*;
use pretty_assertions::assert_eq;

fn assignment() -> ActorAssignment {
    ActorAssignment {
        schema_version: 2,
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
        &[context_file()],
        &[],
    )
    .unwrap();
    let system_content = body["messages"][0]["content"].as_str().unwrap();
    let assignment_json = serde_json::to_string(&assignment).unwrap();
    assert!(system_content.ends_with(&assignment_json));
    assert!(system_content.contains("use add only for a new path absent from context_files"));
    assert!(system_content.contains("never use add for a path present in context_files"));
    assert!(system_content.contains("copy its supplied sha256 into context_sha256"));
    assert_eq!(body["messages"][0]["role"], "system");
    assert!(!system_content.contains("fn parse"));
    assert_eq!(body["messages"][1]["role"], "user");
    let user_content: serde_json::Value =
        serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
    assert_eq!(user_content["context_files"][0]["path"], "src/parser.rs");
    assert_eq!(
        user_content["context_files"][0]["sha256"],
        context_file().content_sha256()
    );
    assert_eq!(body["model"], "qwen/qwen3-coder-30b");
    let edit_variants =
        body["response_format"]["json_schema"]["schema"]["definitions"]["ActorEdit"]["oneOf"]
            .as_array()
            .expect("ActorEdit variants");
    assert_eq!(edit_variants.len(), 1);
    assert_eq!(
        edit_variants[0]["properties"]["kind"]["enum"],
        serde_json::json!(["replace"])
    );
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

#[test]
fn actor_operations_require_matching_planner_approved_tools() {
    let mut result = ActorResult {
        schema_version: 2,
        task_id: "task-1".to_string(),
        proposed_edits: vec![ActorEdit::Add {
            path: "src/parser.rs".to_string(),
            content: "test\n".to_string(),
        }],
        test_commands: vec!["cargo test parser".to_string()],
        diagnostics: Vec::new(),
        needs_escalation: false,
    };
    let assignment = assignment();
    assert!(
        validate_actor_result(&assignment, &[], &result)
            .unwrap_err()
            .to_string()
            .contains("exec_command")
    );

    result.test_commands.clear();
    assert!(validate_actor_result(&assignment, &[], &result).is_ok());

    let mut no_patch_access = assignment;
    no_patch_access.allowed_tools = vec!["exec_command".to_string()];
    no_patch_access.tool_call_map[0].tool = "exec_command".to_string();
    assert!(
        validate_actor_result(&no_patch_access, &[], &result)
            .unwrap_err()
            .to_string()
            .contains("apply_patch")
    );
}

#[test]
fn add_rejects_a_path_already_supplied_as_context() {
    let context = context_file();
    let result = ActorResult {
        schema_version: 2,
        task_id: "task-1".to_string(),
        proposed_edits: vec![ActorEdit::Add {
            path: context.path.clone(),
            content: "replacement disguised as an add\n".to_string(),
        }],
        test_commands: Vec::new(),
        diagnostics: Vec::new(),
        needs_escalation: false,
    };

    let error = validate_actor_result(&assignment(), &[context], &result).unwrap_err();
    assert!(error.to_string().contains("use replace"));
}

#[test]
fn replacement_requires_current_complete_unique_context() {
    let context = context_file();
    let mut result = ActorResult {
        schema_version: 2,
        task_id: "task-1".to_string(),
        proposed_edits: vec![ActorEdit::Replace {
            path: context.path.clone(),
            context_sha256: context.content_sha256(),
            old_text: "old()".to_string(),
            new_text: "new()".to_string(),
        }],
        test_commands: Vec::new(),
        diagnostics: Vec::new(),
        needs_escalation: false,
    };

    assert!(validate_actor_result(&assignment(), std::slice::from_ref(&context), &result).is_ok());

    let ActorEdit::Replace { context_sha256, .. } = &mut result.proposed_edits[0] else {
        panic!("expected replacement edit");
    };
    *context_sha256 = "stale".to_string();
    assert!(
        validate_actor_result(&assignment(), &[context], &result)
            .unwrap_err()
            .to_string()
            .contains("stale context hash")
    );
}

#[test]
fn truncated_context_uses_anchored_excerpt_replacement_schema() {
    let mut context = context_file();
    context.truncated = true;
    let body = actor_request_body(
        &Url::parse("http://127.0.0.1:1234/v1").unwrap(),
        "qwen/qwen3-coder-30b",
        &assignment(),
        std::slice::from_ref(&context),
        &[],
    )
    .unwrap();
    let edit_variants =
        body["response_format"]["json_schema"]["schema"]["definitions"]["ActorEdit"]["oneOf"]
            .as_array()
            .expect("ActorEdit variants");

    assert_eq!(edit_variants.len(), 1);
    assert_eq!(
        edit_variants[0]["properties"]["kind"]["enum"],
        serde_json::json!(["replace_excerpt"])
    );
}

#[test]
fn excerpt_replacement_requires_current_unique_anchored_context() {
    let mut context = context_file();
    context.truncated = true;
    let result = ActorResult {
        schema_version: 2,
        task_id: "task-1".to_string(),
        proposed_edits: vec![ActorEdit::ReplaceExcerpt {
            path: context.path.clone(),
            context_sha256: context.content_sha256(),
            before_anchor: "{ ".to_string(),
            old_text: "old()".to_string(),
            after_anchor: "; }".to_string(),
            new_text: "new()".to_string(),
        }],
        test_commands: Vec::new(),
        diagnostics: Vec::new(),
        needs_escalation: false,
    };

    assert!(validate_actor_result(&assignment(), &[context], &result).is_ok());
}

fn context_file() -> ActorContextFile {
    ActorContextFile {
        path: "src/parser.rs".to_string(),
        content: "fn parse() { old(); }\n".to_string(),
        truncated: false,
    }
}

#[test]
fn actor_usage_accepts_complete_nonnegative_counters() {
    let response = serde_json::json!({
        "usage": {
            "prompt_tokens": 120,
            "completion_tokens": 30,
            "total_tokens": 150,
            "prompt_tokens_details": {"cached_tokens": 10}
        }
    });

    assert_eq!(
        actor_usage(&response),
        Some(ActorUsage {
            prompt_tokens: 120,
            completion_tokens: 30,
            total_tokens: 150,
        })
    );
}

#[test]
fn actor_usage_ignores_missing_or_malformed_counters() {
    for response in [
        serde_json::json!({}),
        serde_json::json!({"usage": null}),
        serde_json::json!({"usage": {"prompt_tokens": 1, "completion_tokens": 2}}),
        serde_json::json!({
            "usage": {"prompt_tokens": -1, "completion_tokens": 2, "total_tokens": 1}
        }),
        serde_json::json!({
            "usage": {"prompt_tokens": "1", "completion_tokens": 2, "total_tokens": 3}
        }),
    ] {
        assert_eq!(actor_usage(&response), None);
    }
}
