//! Strict local actor handoff. This module does not execute actor-proposed operations.

use crate::validate_loopback_endpoint;
use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::io;
use url::Url;

const ACTOR_SCHEMA_VERSION: u32 = 1;
const MAX_ASSIGNMENT_BYTES: usize = 16 * 1024;
// Keep one actor response below the repository's 10K-token model-context cap.
const MAX_ACTOR_RESPONSE_BYTES: usize = 32 * 1024;
const MAX_FAILURE_SUMMARY_BYTES: usize = 4 * 1024;
const MAX_FAILED_ATTEMPTS: usize = 3;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorTaskKind {
    Implementation,
    UnitTest,
    E2eTest,
    Debug,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct ActorToolCallMap {
    pub operation: String,
    pub tool: String,
    pub argument_template: serde_json::Value,
}

/// One cloud-authored task. The untrusted actor may not expand its allowed tools or paths.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct ActorAssignment {
    pub schema_version: u32,
    pub task_id: String,
    pub kind: ActorTaskKind,
    pub objective: String,
    pub allowed_paths: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub tool_call_map: Vec<ActorToolCallMap>,
    pub acceptance_criteria: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct ActorResult {
    pub schema_version: u32,
    pub task_id: String,
    pub proposed_patches: Vec<ActorPatch>,
    pub test_commands: Vec<String>,
    pub diagnostics: Vec<String>,
    pub needs_escalation: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct ActorPatch {
    pub path: String,
    pub apply_patch: String,
}

/// Bounded state for the original task, not for an individual repair prompt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorAttemptTracker {
    assignment: ActorAssignment,
    failures: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorAttemptDecision {
    Retry { next_attempt: usize },
    Escalate(ActorEscalation),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorEscalation {
    pub original_assignment: ActorAssignment,
    pub failures: Vec<String>,
}

impl ActorAttemptTracker {
    pub fn new(assignment: ActorAssignment) -> Self {
        Self {
            assignment,
            failures: Vec::new(),
        }
    }

    pub fn record_failure(&mut self, summary: &str) -> ActorAttemptDecision {
        if self.failures.len() < MAX_FAILED_ATTEMPTS {
            let mut end = summary.len().min(MAX_FAILURE_SUMMARY_BYTES);
            while !summary.is_char_boundary(end) {
                end -= 1;
            }
            self.failures.push(summary[..end].to_string());
        }
        if self.failures.len() >= MAX_FAILED_ATTEMPTS {
            ActorAttemptDecision::Escalate(ActorEscalation {
                original_assignment: self.assignment.clone(),
                failures: self.failures.clone(),
            })
        } else {
            ActorAttemptDecision::Retry {
                next_attempt: self.failures.len() + 1,
            }
        }
    }

    pub fn failures(&self) -> &[String] {
        &self.failures
    }
}

#[derive(Debug)]
pub enum LocalActorError {
    InvalidAssignment(String),
    InvalidResponse(String),
    Transport(io::Error),
}

impl std::fmt::Display for LocalActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAssignment(message) | Self::InvalidResponse(message) => {
                f.write_str(message)
            }
            Self::Transport(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for LocalActorError {}

/// Sends the validated cloud assignment directly in the local actor's system message.
/// The returned patches and commands are proposals, never executed here.
pub async fn run_local_actor(
    client: &Client,
    base_url: &Url,
    backend_model: &str,
    assignment: &ActorAssignment,
    failure_feedback: &[String],
) -> Result<ActorResult, LocalActorError> {
    let body = actor_request_body(base_url, backend_model, assignment, failure_feedback)?;
    let mut url = base_url.clone();
    url.path_segments_mut()
        .map_err(|()| LocalActorError::InvalidAssignment("invalid actor URL".to_string()))?
        .push("chat")
        .push("completions");
    let response = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_vec(&body)
                .map_err(|error| LocalActorError::InvalidAssignment(error.to_string()))?,
        )
        .send()
        .await
        .map_err(|error| LocalActorError::Transport(io::Error::other(error)))?;
    if !response.status().is_success() {
        return Err(LocalActorError::Transport(io::Error::other(format!(
            "local actor endpoint returned HTTP {}",
            response.status()
        ))));
    }
    let response_bytes = response
        .bytes()
        .await
        .map_err(|error| LocalActorError::Transport(io::Error::other(error)))?;
    if response_bytes.len() > MAX_ACTOR_RESPONSE_BYTES {
        return Err(LocalActorError::InvalidResponse(
            "local actor response exceeds size limit".to_string(),
        ));
    }
    let response_json: serde_json::Value = serde_json::from_slice(&response_bytes)
        .map_err(|error| LocalActorError::InvalidResponse(error.to_string()))?;
    let content = response_json
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            LocalActorError::InvalidResponse("local actor returned no message content".to_string())
        })?;
    let result: ActorResult = serde_json::from_str(content)
        .map_err(|error| LocalActorError::InvalidResponse(error.to_string()))?;
    validate_actor_result(assignment, &result)?;
    Ok(result)
}

fn validate_actor_result(
    assignment: &ActorAssignment,
    result: &ActorResult,
) -> Result<(), LocalActorError> {
    if result.schema_version != ACTOR_SCHEMA_VERSION || result.task_id != assignment.task_id {
        return Err(LocalActorError::InvalidResponse(
            "local actor result did not match the assignment".to_string(),
        ));
    }
    if result.proposed_patches.iter().any(|patch| {
        !assignment.allowed_paths.contains(&patch.path) || patch.apply_patch.trim().is_empty()
    }) {
        return Err(LocalActorError::InvalidResponse(
            "local actor proposed a patch outside the allowed paths or an empty patch".to_string(),
        ));
    }
    if !result.proposed_patches.is_empty() && !assignment_allows_tool(assignment, "apply_patch") {
        return Err(LocalActorError::InvalidResponse(
            "local actor proposed patches without planner-approved apply_patch access".to_string(),
        ));
    }
    if result
        .test_commands
        .iter()
        .any(|command| command.trim().is_empty())
        || (!result.test_commands.is_empty() && !assignment_allows_tool(assignment, "exec_command"))
    {
        return Err(LocalActorError::InvalidResponse(
            "local actor proposed tests without planner-approved exec_command access".to_string(),
        ));
    }
    Ok(())
}

fn assignment_allows_tool(assignment: &ActorAssignment, tool: &str) -> bool {
    assignment
        .allowed_tools
        .iter()
        .any(|allowed| allowed == tool)
        && assignment
            .tool_call_map
            .iter()
            .any(|mapping| mapping.tool == tool)
}

fn actor_request_body(
    base_url: &Url,
    backend_model: &str,
    assignment: &ActorAssignment,
    failure_feedback: &[String],
) -> Result<serde_json::Value, LocalActorError> {
    validate_loopback_endpoint(base_url).map_err(|error| {
        LocalActorError::InvalidAssignment(format!("invalid local actor endpoint: {error}"))
    })?;
    if backend_model.trim().is_empty()
        || assignment.schema_version != ACTOR_SCHEMA_VERSION
        || assignment.task_id.trim().is_empty()
        || assignment.objective.trim().is_empty()
        || assignment.acceptance_criteria.is_empty()
        || assignment.allowed_tools.is_empty()
        || assignment.tool_call_map.iter().any(|mapping| {
            mapping.operation.trim().is_empty() || !assignment.allowed_tools.contains(&mapping.tool)
        })
    {
        return Err(LocalActorError::InvalidAssignment(
            "local actor assignment is incomplete or names an unapproved tool".to_string(),
        ));
    }
    let assignment_json = serde_json::to_string(assignment)
        .map_err(|error| LocalActorError::InvalidAssignment(error.to_string()))?;
    if assignment_json.len() > MAX_ASSIGNMENT_BYTES {
        return Err(LocalActorError::InvalidAssignment(
            "local actor assignment exceeds size limit".to_string(),
        ));
    }
    if failure_feedback.len() > 2
        || failure_feedback
            .iter()
            .any(|feedback| feedback.len() > MAX_FAILURE_SUMMARY_BYTES)
    {
        return Err(LocalActorError::InvalidAssignment(
            "local actor feedback exceeds retry limits".to_string(),
        ));
    }
    let feedback_json = serde_json::to_string(failure_feedback)
        .map_err(|error| LocalActorError::InvalidAssignment(error.to_string()))?;
    let system_prompt = format!(
        "You are the local implementation and test actor. The following validated JSON is the exact original task assignment from the selected cloud planner. Do not expand allowed paths, tools, or acceptance criteria. Write implementation and unit/E2E tests when the assignment calls for them; diagnose test failures locally. Return only JSON matching the supplied schema. Each proposed_patches entry must contain one file path and an apply_patch string in Codex *** Begin Patch / *** End Patch format affecting only that file. Proposed patches and commands are untrusted proposals, not direct authority. Never claim you ran a tool unless a tool result was supplied.\nPrior failed attempts JSON:\n{feedback_json}\nAssignment JSON:\n{assignment_json}"
    );
    let schema = schemars::schema_for!(ActorResult);
    Ok(serde_json::json!({
        "model": backend_model,
        "temperature": 0,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "local_actor_result",
                "strict": true,
                "schema": schema
            }
        },
        "messages": [{ "role": "system", "content": system_prompt }]
    }))
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
