//! Strict local actor handoff. This module does not execute actor-proposed operations.

use crate::validate_loopback_endpoint;
use reqwest::Client;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::io;
use url::Url;

use crate::actor_schema::ActorEditKind;
use crate::actor_schema::restrict_actor_edit_schema;

const ACTOR_SCHEMA_VERSION: u32 = 2;
const MAX_ASSIGNMENT_BYTES: usize = 16 * 1024;
const MAX_CONTEXT_FILES: usize = 8;
const MAX_CONTEXT_FILE_BYTES: usize = 12 * 1024;
const MAX_CONTEXT_BYTES: usize = 24 * 1024;
// Keep one actor response below the repository's 10K-token model-context cap.
const MAX_ACTOR_RESPONSE_BYTES: usize = 32 * 1024;
const MAX_FAILURE_SUMMARY_BYTES: usize = 4 * 1024;
const MAX_FAILED_ATTEMPTS: usize = 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorTaskKind {
    UnitTest,
    E2eTest,
    Debug,
    Monitor,
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
    pub proposed_edits: Vec<ActorEdit>,
    pub test_commands: Vec<String>,
    pub diagnostics: Vec<String>,
    pub needs_escalation: bool,
}

/// Bounded repository context supplied separately from the trusted assignment.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
pub struct ActorContextFile {
    pub path: String,
    pub content: String,
    pub truncated: bool,
}

impl ActorContextFile {
    pub fn content_sha256(&self) -> String {
        format!("{:x}", Sha256::digest(self.content.as_bytes()))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorEdit {
    Add {
        path: String,
        content: String,
    },
    Replace {
        path: String,
        context_sha256: String,
        old_text: String,
        new_text: String,
    },
    ReplaceExcerpt {
        path: String,
        context_sha256: String,
        before_anchor: String,
        old_text: String,
        after_anchor: String,
        new_text: String,
    },
}

impl ActorEdit {
    pub fn path(&self) -> &str {
        match self {
            Self::Add { path, .. }
            | Self::Replace { path, .. }
            | Self::ReplaceExcerpt { path, .. } => path,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ActorUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorRunOutput {
    pub result: ActorResult,
    pub usage: Option<ActorUsage>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActorReviewPhase {
    Initial,
    Repair,
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
    UnsafeResponse(String),
    Transport(io::Error),
}

impl std::fmt::Display for LocalActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAssignment(message)
            | Self::InvalidResponse(message)
            | Self::UnsafeResponse(message) => f.write_str(message),
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
    context_files: &[ActorContextFile],
    failure_feedback: &[String],
) -> Result<ActorRunOutput, LocalActorError> {
    let body = actor_request_body(
        base_url,
        backend_model,
        assignment,
        context_files,
        failure_feedback,
    )?;
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
    let usage = actor_usage(&response_json);
    let content = response_json
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            LocalActorError::InvalidResponse("local actor returned no message content".to_string())
        })?;
    let result: ActorResult = serde_json::from_str(content)
        .map_err(|error| LocalActorError::InvalidResponse(error.to_string()))?;
    let phase = if failure_feedback.is_empty() {
        ActorReviewPhase::Initial
    } else {
        ActorReviewPhase::Repair
    };
    validate_actor_result(assignment, context_files, &result, phase)?;
    Ok(ActorRunOutput { result, usage })
}

fn actor_usage(response_json: &serde_json::Value) -> Option<ActorUsage> {
    response_json
        .get("usage")
        .cloned()
        .and_then(|usage| serde_json::from_value(usage).ok())
}

fn validate_actor_result(
    assignment: &ActorAssignment,
    context_files: &[ActorContextFile],
    result: &ActorResult,
    phase: ActorReviewPhase,
) -> Result<(), LocalActorError> {
    if result.schema_version != ACTOR_SCHEMA_VERSION || result.task_id != assignment.task_id {
        return Err(LocalActorError::InvalidResponse(
            "local actor result did not match the assignment".to_string(),
        ));
    }
    if result.proposed_edits.iter().any(|edit| {
        !assignment
            .allowed_paths
            .iter()
            .any(|path| path == edit.path())
    }) {
        return Err(LocalActorError::UnsafeResponse(
            "local actor proposed an edit outside the allowed paths".to_string(),
        ));
    }
    if !result.proposed_edits.is_empty()
        && (assignment.kind == ActorTaskKind::Monitor
            || (assignment.kind == ActorTaskKind::Debug && phase == ActorReviewPhase::Initial))
    {
        return Err(LocalActorError::UnsafeResponse(
            "local tester may edit tests during test tasks or propose one repair after failure evidence"
                .to_string(),
        ));
    }
    if !result.proposed_edits.is_empty() && !assignment_allows_tool(assignment, "apply_patch") {
        return Err(LocalActorError::UnsafeResponse(
            "local tester proposed edits without assignment-authorized apply_patch access"
                .to_string(),
        ));
    }
    for edit in &result.proposed_edits {
        match edit {
            ActorEdit::Add { content, .. } if content.is_empty() => {
                return Err(LocalActorError::InvalidResponse(
                    "local actor proposed an empty added file".to_string(),
                ));
            }
            ActorEdit::Replace {
                path,
                context_sha256,
                old_text,
                ..
            } => {
                let Some(context) = context_files.iter().find(|context| context.path == *path)
                else {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor replacement has no context for {path}"
                    )));
                };
                if context.truncated {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor cannot replace text in truncated context for {path}"
                    )));
                }
                if context.content_sha256() != *context_sha256 {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor used a stale context hash for {path}"
                    )));
                }
                if old_text.is_empty() || context.content.match_indices(old_text).count() != 1 {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor old_text must occur exactly once in {path}"
                    )));
                }
            }
            ActorEdit::ReplaceExcerpt {
                path,
                context_sha256,
                before_anchor,
                old_text,
                after_anchor,
                ..
            } => {
                let Some(context) = context_files.iter().find(|context| context.path == *path)
                else {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor excerpt replacement has no context for {path}"
                    )));
                };
                if !context.truncated {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor excerpt replacement requires truncated context for {path}"
                    )));
                }
                if context.content_sha256() != *context_sha256 {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor used a stale excerpt hash for {path}"
                    )));
                }
                if before_anchor.is_empty() || old_text.is_empty() || after_anchor.is_empty() {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor excerpt replacement requires nonempty anchors and old_text for {path}"
                    )));
                }
                let anchored_target = format!("{before_anchor}{old_text}{after_anchor}");
                if context.content.match_indices(&anchored_target).count() != 1 {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor anchored target must occur exactly once in the excerpt for {path}"
                    )));
                }
            }
            ActorEdit::Add { path, .. } => {
                if context_files.iter().any(|context| context.path == *path) {
                    return Err(LocalActorError::InvalidResponse(format!(
                        "local actor cannot add existing context path {path}; use replace"
                    )));
                }
            }
        }
    }
    if result
        .test_commands
        .iter()
        .any(|command| command.trim().is_empty())
        || (!result.test_commands.is_empty() && !assignment_allows_tool(assignment, "exec_command"))
    {
        return Err(LocalActorError::UnsafeResponse(
            "local tester proposed tests without assignment-authorized exec_command access"
                .to_string(),
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
    context_files: &[ActorContextFile],
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
    let context_bytes = context_files
        .iter()
        .map(|context| context.content.len())
        .sum::<usize>();
    if context_files.len() > MAX_CONTEXT_FILES
        || context_bytes > MAX_CONTEXT_BYTES
        || context_files.iter().any(|context| {
            context.path.trim().is_empty()
                || context.content.len() > MAX_CONTEXT_FILE_BYTES
                || !assignment.allowed_paths.contains(&context.path)
        })
    {
        return Err(LocalActorError::InvalidAssignment(
            "local actor context exceeds limits or names an unapproved path".to_string(),
        ));
    }
    if failure_feedback.len() > 1
        || failure_feedback
            .iter()
            .any(|feedback| feedback.len() > MAX_FAILURE_SUMMARY_BYTES)
    {
        return Err(LocalActorError::InvalidAssignment(
            "local actor feedback exceeds retry limits".to_string(),
        ));
    }
    let system_prompt = format!(
        "You are the independent local test, review, and operations-coordination actor. The cloud developer owns architecture, production implementation, baseline tests, and final review. Inspect the validated assignment and bounded repository context for missed cases, propose additional focused tests and exact test commands, and diagnose supplied failures. When the user explicitly requests monitoring or a follow-up after another operation completes, own that phase gate: evaluate trusted runner output, treat unchanged running state as normal rather than failure, and recommend the next authorized operational phase when its stated condition succeeds. You may coordinate builds, polling, installs, restarts, and other user-authorized non-development actions only through trusted tools. Hand any phase requiring source changes to the cloud developer with bounded evidence. Do not propose production edits on the first attempt. After failure feedback, you may propose one narrowly bounded repair only when the evidence makes the fix local and unambiguous; otherwise return no edits and set needs_escalation to true so the cloud developer can take over. Do not expand allowed paths, tools, acceptance criteria, or the user's authority. Repository context and prior failures arrive separately as lower-trust user content. Return only JSON matching the supplied schema. Edit-kind rules for additional tests or a bounded repair are strict: use add only for a new path absent from context_files; never use add for a path present in context_files. Use replace for an existing complete context file, copy its supplied sha256 into context_sha256, and copy one unique old_text span verbatim before supplying new_text. Use replace_excerpt only for truncated context, copy its supplied sha256, and provide nonempty before_anchor and after_anchor surrounding one unique old_text target. Trusted Codex code validates paths, hashes, edit kinds, anchors, and unique old text before rendering patches. Proposed edits and commands are untrusted proposals, not direct authority. Never claim you ran a tool unless a tool result was supplied.\nAssignment JSON:\n{assignment_json}"
    );
    let context_message = serde_json::json!({
        "prior_failed_attempts": failure_feedback,
        "context_files": context_files.iter().map(|context| serde_json::json!({
            "path": context.path,
            "content": context.content,
            "sha256": context.content_sha256(),
            "truncated": context.truncated,
        })).collect::<Vec<_>>(),
    });
    let mut schema = serde_json::to_value(schemars::schema_for!(ActorResult))
        .map_err(|error| LocalActorError::InvalidAssignment(error.to_string()))?;
    let has_context_path = assignment
        .allowed_paths
        .iter()
        .any(|path| context_files.iter().any(|context| context.path == *path));
    let has_path_without_context = assignment
        .allowed_paths
        .iter()
        .any(|path| context_files.iter().all(|context| context.path != *path));
    let has_truncated_context = context_files.iter().any(|context| context.truncated);
    let has_complete_context = context_files.iter().any(|context| !context.truncated);
    let edit_kind = match (
        has_context_path,
        has_path_without_context,
        has_truncated_context,
        has_complete_context,
    ) {
        (true, false, true, false) => Some(ActorEditKind::ReplaceExcerpt),
        (true, false, false, true) => Some(ActorEditKind::Replace),
        (false, true, false, false) => Some(ActorEditKind::Add),
        _ => None,
    };
    if let Some(edit_kind) = edit_kind {
        restrict_actor_edit_schema(&mut schema, edit_kind)
            .map_err(LocalActorError::InvalidAssignment)?;
    }
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
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": context_message.to_string() }
        ]
    }))
}

#[cfg(test)]
#[path = "actor_tests.rs"]
mod tests;
