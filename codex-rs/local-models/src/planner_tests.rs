use super::*;

fn request() -> PlannerRequest {
    PlannerRequest {
        schema_version: 1,
        phase: PlannerPhase::Plan,
        objective: "Add bounded local planning".to_string(),
        constraints: vec!["Cloud worker owns all edits".to_string()],
        context_files: vec![PlannerContextFile {
            path: "src/lib.rs".to_string(),
            content: "pub fn existing() {}".to_string(),
            truncated: false,
        }],
    }
}

#[test]
fn request_caps_reasoning_and_output() {
    let body = planner_request_body(
        &Url::parse("http://127.0.0.1:1234/v1").unwrap(),
        "qwen",
        &request(),
    )
    .unwrap();

    assert_eq!(
        body.get("reasoning_budget_tokens"),
        Some(&serde_json::json!(1_500))
    );
    assert_eq!(body.get("max_tokens"), Some(&serde_json::json!(8_192)));
    assert_eq!(
        body.pointer("/response_format/json_schema/name"),
        Some(&serde_json::json!("local_coordinator_result"))
    );
}

#[test]
fn result_enforces_local_and_cloud_responsibilities() {
    let valid_local = PlannerStep {
        id: "verify".to_string(),
        owner: PlannerStepOwner::Local,
        kind: PlannerStepKind::Verify,
        objective: "Run focused tests".to_string(),
        rationale: "Verification is repetitive".to_string(),
        files: Vec::new(),
        acceptance_criteria: vec!["Tests pass".to_string()],
        suggested_command: Some("just test -p codex-tui usage_hud".to_string()),
    };
    assert!(valid_step_ownership(&valid_local));

    let invalid_local = PlannerStep {
        owner: PlannerStepOwner::Local,
        kind: PlannerStepKind::Implement,
        suggested_command: None,
        ..valid_local.clone()
    };
    assert!(!valid_step_ownership(&invalid_local));

    let invalid_cloud = PlannerStep {
        owner: PlannerStepOwner::Cloud,
        kind: PlannerStepKind::TestAuthoring,
        suggested_command: Some("write a test".to_string()),
        ..valid_local
    };
    assert!(!valid_step_ownership(&invalid_cloud));
}

#[test]
fn request_rejects_remote_endpoint() {
    let error = planner_request_body(
        &Url::parse("https://example.com/v1").unwrap(),
        "qwen",
        &request(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("loopback"));
}

#[test]
fn result_requires_context_paths_when_more_context_is_needed() {
    let result = PlannerResult {
        schema_version: 1,
        summary: "Need the caller".to_string(),
        steps: Vec::new(),
        risks: Vec::new(),
        verification: Vec::new(),
        needs_more_context: true,
        requested_context_paths: Vec::new(),
    };

    assert!(validate_planner_result(&result).is_err());
}
