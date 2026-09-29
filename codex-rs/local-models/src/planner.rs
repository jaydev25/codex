//! Bounded local planning consultation for a cloud implementation agent.

use crate::validate_loopback_endpoint;
use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::io;
use url::Url;

const PLANNER_SCHEMA_VERSION: u32 = 1;
const MAX_OBJECTIVE_BYTES: usize = 16 * 1024;
const MAX_CONSTRAINTS: usize = 32;
const MAX_CONTEXT_FILES: usize = 16;
const MAX_CONTEXT_FILE_BYTES: usize = 16 * 1024;
const MAX_CONTEXT_BYTES: usize = 64 * 1024;
const MAX_PLANNER_RESPONSE_BYTES: usize = 40 * 1024;
const PLANNER_MAX_OUTPUT_TOKENS: u32 = 8_192;
const PLANNER_REASONING_BUDGET_TOKENS: u32 = 1_500;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct PlannerRequest {
    pub schema_version: u32,
    pub phase: PlannerPhase,
    pub objective: String,
    pub constraints: Vec<String>,
    pub context_files: Vec<PlannerContextFile>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerPhase {
    Plan,
    Verify,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct PlannerContextFile {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct PlannerStep {
    pub id: String,
    pub owner: PlannerStepOwner,
    pub kind: PlannerStepKind,
    pub objective: String,
    pub rationale: String,
    pub files: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub suggested_command: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerStepOwner {
    Local,
    Cloud,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerStepKind {
    Inspect,
    Implement,
    TestAuthoring,
    Verify,
    Triage,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct PlannerResult {
    pub schema_version: u32,
    pub summary: String,
    pub steps: Vec<PlannerStep>,
    pub risks: Vec<String>,
    pub verification: Vec<String>,
    pub needs_more_context: bool,
    pub requested_context_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PlannerUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannerRunOutput {
    pub result: PlannerResult,
    pub usage: Option<PlannerUsage>,
}

#[derive(Debug)]
pub enum LocalPlannerError {
    InvalidRequest(String),
    InvalidResponse(String),
    Transport(io::Error),
}

impl std::fmt::Display for LocalPlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(message) | Self::InvalidResponse(message) => f.write_str(message),
            Self::Transport(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for LocalPlannerError {}

/// Asks the configured loopback model for a plan. The model receives no tools
/// and the result is advisory input for the cloud implementation agent.
pub async fn run_local_planner(
    client: &Client,
    base_url: &Url,
    backend_model: &str,
    request: &PlannerRequest,
) -> Result<PlannerRunOutput, LocalPlannerError> {
    let body = planner_request_body(base_url, backend_model, request)?;
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|()| LocalPlannerError::InvalidRequest("invalid planner URL".to_string()))?
        .push("chat")
        .push("completions");
    let response = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_vec(&body)
                .map_err(|error| LocalPlannerError::InvalidRequest(error.to_string()))?,
        )
        .send()
        .await
        .map_err(|error| LocalPlannerError::Transport(io::Error::other(error)))?;
    if !response.status().is_success() {
        return Err(LocalPlannerError::Transport(io::Error::other(format!(
            "local planner endpoint returned HTTP {}",
            response.status()
        ))));
    }
    let response_bytes = response
        .bytes()
        .await
        .map_err(|error| LocalPlannerError::Transport(io::Error::other(error)))?;
    if response_bytes.len() > MAX_PLANNER_RESPONSE_BYTES {
        return Err(LocalPlannerError::InvalidResponse(
            "local planner response exceeds size limit".to_string(),
        ));
    }
    let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)
        .map_err(|error| LocalPlannerError::InvalidResponse(error.to_string()))?;
    let usage = response_json
        .get("usage")
        .cloned()
        .and_then(|usage| serde_json::from_value(usage).ok());
    let content = response_json
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            LocalPlannerError::InvalidResponse(
                "local planner returned no message content".to_string(),
            )
        })?;
    let result: PlannerResult = serde_json::from_str(content)
        .map_err(|error| LocalPlannerError::InvalidResponse(error.to_string()))?;
    validate_planner_result(&result)?;
    Ok(PlannerRunOutput { result, usage })
}

fn validate_planner_result(result: &PlannerResult) -> Result<(), LocalPlannerError> {
    let invalid_step = result.steps.iter().any(|step| {
        step.id.trim().is_empty()
            || step.objective.trim().is_empty()
            || step.acceptance_criteria.is_empty()
            || !valid_step_ownership(step)
    });
    if result.schema_version != PLANNER_SCHEMA_VERSION
        || result.summary.trim().is_empty()
        || invalid_step
        || (result.needs_more_context && result.requested_context_paths.is_empty())
    {
        return Err(LocalPlannerError::InvalidResponse(
            "local planner returned an incomplete plan".to_string(),
        ));
    }
    Ok(())
}

fn valid_step_ownership(step: &PlannerStep) -> bool {
    match (step.owner, step.kind, step.suggested_command.as_deref()) {
        (
            PlannerStepOwner::Cloud,
            PlannerStepKind::Implement | PlannerStepKind::TestAuthoring,
            None,
        ) => true,
        (
            PlannerStepOwner::Local,
            PlannerStepKind::Inspect | PlannerStepKind::Verify,
            Some(command),
        ) => !command.trim().is_empty(),
        (PlannerStepOwner::Local, PlannerStepKind::Triage, None) => true,
        _ => false,
    }
}

fn planner_request_body(
    base_url: &Url,
    backend_model: &str,
    request: &PlannerRequest,
) -> Result<serde_json::Value, LocalPlannerError> {
    validate_loopback_endpoint(base_url).map_err(|error| {
        LocalPlannerError::InvalidRequest(format!("invalid local planner endpoint: {error}"))
    })?;
    let context_bytes = request
        .context_files
        .iter()
        .map(|file| file.content.len())
        .sum::<usize>();
    if backend_model.trim().is_empty()
        || request.schema_version != PLANNER_SCHEMA_VERSION
        || request.objective.trim().is_empty()
        || request.objective.len() > MAX_OBJECTIVE_BYTES
        || request.constraints.len() > MAX_CONSTRAINTS
        || request.context_files.len() > MAX_CONTEXT_FILES
        || context_bytes > MAX_CONTEXT_BYTES
        || request
            .context_files
            .iter()
            .any(|file| file.path.trim().is_empty() || file.content.len() > MAX_CONTEXT_FILE_BYTES)
    {
        return Err(LocalPlannerError::InvalidRequest(
            "local planner request is incomplete or exceeds its bounds".to_string(),
        ));
    }
    let schema = serde_json::to_value(schemars::schema_for!(PlannerResult))
        .map_err(|error| LocalPlannerError::InvalidRequest(error.to_string()))?;
    let system_prompt = "You are the local coordinator of a coding agent. Keep the expensive cloud worker focused on architecture, source-code changes, and authoring tests. You own decomposition, repetitive repository inspection, selection of focused verification commands, and triage of supplied test evidence. Cloud-owned steps may only be implement or test_authoring and must not contain commands. Local-owned inspect and verify steps must provide a read-only or verification command for the host to execute through its normal permission-aware tools; local triage steps contain no command. Never write source code, patches, or test code, and never claim a command ran unless its output is present in the supplied context. In verify phase, assess the supplied evidence and request cloud repair only when behavior or acceptance criteria are not satisfied. Keep output bounded and request only materially necessary missing paths. Return only JSON matching the supplied schema.";
    let user_message = serde_json::to_string(request)
        .map_err(|error| LocalPlannerError::InvalidRequest(error.to_string()))?;
    Ok(serde_json::json!({
        "model": backend_model,
        "temperature": 0,
        "max_tokens": PLANNER_MAX_OUTPUT_TOKENS,
        "reasoning_budget_tokens": PLANNER_REASONING_BUDGET_TOKENS,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "local_coordinator_result",
                "strict": true,
                "schema": schema
            }
        },
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_message }
        ]
    }))
}

#[cfg(test)]
#[path = "planner_tests.rs"]
mod tests;
