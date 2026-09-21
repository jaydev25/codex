//! Read-only bridge from the cloud planner to the configured local actor.
//! Actor proposals are returned to the planner; this tool never applies or executes them.

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_apply_patch::Hunk;
use codex_local_models::ActorAssignment;
use codex_local_models::ActorAttemptDecision;
use codex_local_models::ActorAttemptTracker;
use codex_local_models::LocalActorError;
use codex_local_models::run_local_actor;
use codex_protocol::models::ResponseItem;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use url::Url;

const TOOL_NAME: &str = "local_actor";
const MAX_ACTIVE_TASKS: usize = 64;

#[derive(Default)]
struct ActorRuns(Mutex<HashMap<String, ActorRun>>);

struct ActorRun {
    assignment: ActorAssignment,
    tracker: ActorAttemptTracker,
    in_flight: bool,
    terminal: bool,
}

pub(crate) struct LocalActorHandler;

impl ToolExecutor<ToolInvocation> for LocalActorHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        let string = || JsonSchema::string(None);
        let string_list = || JsonSchema::array(string(), None);
        let tool_map = JsonSchema::object(
            BTreeMap::from([
                ("operation".to_string(), string()),
                ("tool".to_string(), string()),
                (
                    "argument_template".to_string(),
                    JsonSchema::object(BTreeMap::new(), None, Some(true.into())),
                ),
            ]),
            Some(vec![
                "operation".to_string(),
                "tool".to_string(),
                "argument_template".to_string(),
            ]),
            Some(false.into()),
        );
        ToolSpec::Function(ResponsesApiTool {
            name: TOOL_NAME.to_string(),
            description: "Delegate one bounded implementation, unit-test, E2E-test, or debugging assignment to the configured local LM Studio actor. Supply structured JSON, not line-by-line code. After a failed patch or test, call again with the same original assignment and failure_feedback. The third failed attempt returns the original task to the cloud planner without calling the actor. Proposals are untrusted; this tool never applies patches or runs commands."
                .to_string(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::object(
                BTreeMap::from([
                    ("schema_version".to_string(), JsonSchema::integer(None)),
                    ("task_id".to_string(), string()),
                    (
                        "kind".to_string(),
                        JsonSchema::string_enum(
                            vec![
                                json!("implementation"),
                                json!("unit_test"),
                                json!("e2e_test"),
                                json!("debug"),
                            ],
                            None,
                        ),
                    ),
                    ("objective".to_string(), string()),
                    ("allowed_paths".to_string(), string_list()),
                    ("allowed_tools".to_string(), string_list()),
                    (
                        "tool_call_map".to_string(),
                        JsonSchema::array(tool_map, None),
                    ),
                    ("acceptance_criteria".to_string(), string_list()),
                    ("failure_feedback".to_string(), string()),
                ]),
                Some(vec![
                    "schema_version".to_string(),
                    "task_id".to_string(),
                    "kind".to_string(),
                    "objective".to_string(),
                    "allowed_paths".to_string(),
                    "allowed_tools".to_string(),
                    "tool_call_map".to_string(),
                    "acceptance_criteria".to_string(),
                ]),
                Some(false.into()),
            ),
            output_schema: None,
        })
    }

    fn handle<'a>(&'a self, invocation: ToolInvocation) -> codex_tools::ToolExecutorFuture<'a>
    where
        ToolInvocation: 'a,
    {
        Box::pin(async move {
            let ToolPayload::Function { arguments } = &invocation.payload else {
                return Err(FunctionCallError::RespondToModel(
                    "local_actor requires structured function arguments".to_string(),
                ));
            };
            let mut input: serde_json::Value = parse_arguments(arguments)?;
            let feedback = input
                .as_object_mut()
                .and_then(|object| object.remove("failure_feedback"))
                .map(|value| serde_json::from_value::<String>(value).map_err(|error| {
                    FunctionCallError::RespondToModel(format!("invalid failure feedback: {error}"))
                }))
                .transpose()?;
            let assignment: ActorAssignment = serde_json::from_value(input).map_err(|error| {
                FunctionCallError::RespondToModel(format!("invalid actor assignment: {error}"))
            })?;
            let policy = &invocation.turn.config.local_analysis;
            let (Some(base_url), Some(backend_model)) = (
                policy.backend_url.as_deref(),
                policy.backend_model.as_deref(),
            ) else {
                return Err(FunctionCallError::RespondToModel(
                    "local actor backend is not configured".to_string(),
                ));
            };
            let base_url = Url::parse(base_url).map_err(|error| {
                FunctionCallError::RespondToModel(format!("invalid local actor URL: {error}"))
            })?;
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
            let runs = invocation
                .session
                .services
                .thread_extension_data
                .get_or_init(ActorRuns::default);
            let needs_recovery = !runs
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains_key(&assignment.task_id);
            let recovered = if needs_recovery {
                recover_actor_run(&invocation, &assignment.task_id).await
            } else {
                None
            };
            let (attempt, failures) = {
                let mut active = runs.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if !active.contains_key(&assignment.task_id)
                    && let Some(run) = recovered
                {
                    active.insert(assignment.task_id.clone(), run);
                }
                if let Some(run) = active.get_mut(&assignment.task_id) {
                    if run.assignment != assignment || run.in_flight || run.terminal {
                        return Err(FunctionCallError::RespondToModel(
                            "actor task changed its original assignment, is running, or already escalated"
                                .to_string(),
                        ));
                    }
                    let Some(feedback) = feedback.as_deref() else {
                        return Err(FunctionCallError::RespondToModel(
                            "actor retry requires failure_feedback".to_string(),
                        ));
                    };
                    match run.tracker.record_failure(feedback) {
                        ActorAttemptDecision::Retry { next_attempt } => {
                            run.in_flight = true;
                            (next_attempt, run.tracker.failures().to_vec())
                        }
                        ActorAttemptDecision::Escalate(escalation) => {
                            run.terminal = true;
                            let output = serde_json::to_string(&json!({
                                "status": "escalate",
                                "original_assignment": escalation.original_assignment,
                                "failures": escalation.failures,
                            }))
                            .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
                            return Ok(boxed_tool_output(FunctionToolOutput::from_text(
                                output,
                                Some(true),
                            )));
                        }
                    }
                } else {
                    if feedback.is_some() || active.len() >= MAX_ACTIVE_TASKS {
                        return Err(FunctionCallError::RespondToModel(
                            "actor task cannot start with feedback or task limit reached"
                                .to_string(),
                        ));
                    }
                    active.insert(
                        assignment.task_id.clone(),
                        ActorRun {
                            assignment: assignment.clone(),
                            tracker: ActorAttemptTracker::new(assignment.clone()),
                            in_flight: true,
                            terminal: false,
                        },
                    );
                    (1, Vec::new())
                }
            };
            let result = run_local_actor(&client, &base_url, backend_model, &assignment, &failures)
                .await;
            if let Some(run) = runs
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get_mut(&assignment.task_id)
            {
                run.in_flight = false;
            }
            let result = match result {
                Ok(result) => result,
                Err(LocalActorError::InvalidAssignment(message)) => {
                    return Err(FunctionCallError::RespondToModel(message));
                }
                Err(LocalActorError::InvalidResponse(message)) => {
                    if let Some(run) = runs.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get_mut(&assignment.task_id) {
                        run.terminal = true;
                    }
                    let output = serde_json::to_string(&json!({
                        "status": "escalate",
                        "original_assignment": assignment,
                        "failures": failures,
                        "reason": message,
                    }))
                    .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
                    return Ok(boxed_tool_output(FunctionToolOutput::from_text(output, Some(true))));
                }
                Err(LocalActorError::Transport(error)) => {
                    if let Some(run) = runs.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get_mut(&assignment.task_id) {
                        run.terminal = true;
                    }
                    let output = serde_json::to_string(&json!({
                        "status": "escalate",
                        "original_assignment": assignment,
                        "failures": failures,
                        "reason": error.to_string(),
                    }))
                    .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
                    return Ok(boxed_tool_output(FunctionToolOutput::from_text(output, Some(true))));
                }
            };
            if let Err(reason) = validate_actor_patches(&result) {
                if let Some(run) = runs
                    .0
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get_mut(&assignment.task_id)
                {
                    run.terminal = true;
                }
                let output = serde_json::to_string(&json!({
                    "status": "escalate",
                    "original_assignment": assignment,
                    "failures": failures,
                    "reason": reason,
                }))
                .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
                return Ok(boxed_tool_output(FunctionToolOutput::from_text(output, Some(true))));
            }
            let output = serde_json::to_string(&json!({ "status": "proposal", "attempt": attempt, "result": result })).map_err(|error| {
                FunctionCallError::RespondToModel(format!("invalid local actor result: {error}"))
            })?;
            Ok(boxed_tool_output(FunctionToolOutput::from_text(
                output,
                Some(true),
            )))
        })
    }
}

impl CoreToolRuntime for LocalActorHandler {}

fn validate_actor_patches(result: &codex_local_models::ActorResult) -> Result<(), String> {
    for patch in &result.proposed_patches {
        let parsed = codex_apply_patch::parse_patch(&patch.apply_patch)
            .map_err(|error| format!("invalid actor patch proposal: {error}"))?;
        if parsed.hunks.len() != 1
            || parsed.hunks[0].path().to_string_lossy() != patch.path
            || matches!(&parsed.hunks[0], Hunk::UpdateFile { move_path: Some(_), .. })
        {
            return Err("actor patch proposal changes a path outside its declared file".to_string());
        }
    }
    Ok(())
}

async fn recover_actor_run(invocation: &ToolInvocation, task_id: &str) -> Option<ActorRun> {
    let history = invocation.session.clone_history().await;
    let mut run: Option<ActorRun> = None;
    let mut relevant_calls = std::collections::HashSet::new();
    for item in history.raw_items() {
        match item {
            ResponseItem::FunctionCall { name, arguments, call_id, .. }
                if name == TOOL_NAME && call_id != &invocation.call_id =>
            {
                let Ok(mut input) = serde_json::from_str::<serde_json::Value>(arguments) else {
                    continue;
                };
                let feedback = input
                    .as_object_mut()
                    .and_then(|object| object.remove("failure_feedback"))
                    .and_then(|value| value.as_str().map(str::to_owned));
                let Ok(assignment) = serde_json::from_value::<ActorAssignment>(input) else {
                    continue;
                };
                if assignment.task_id != task_id {
                    continue;
                }
                relevant_calls.insert(call_id.as_str());
                match &mut run {
                    None if feedback.is_none() => {
                        run = Some(ActorRun {
                            tracker: ActorAttemptTracker::new(assignment.clone()),
                            assignment,
                            in_flight: false,
                            terminal: false,
                        });
                    }
                    Some(existing) if existing.assignment == assignment => {
                        if let Some(feedback) = feedback {
                            existing.tracker.record_failure(&feedback);
                        }
                    }
                    _ => return None,
                }
            }
            ResponseItem::FunctionCallOutput { call_id: Some(call_id), output, .. }
                if relevant_calls.contains(call_id.as_str()) =>
            {
                if output
                    .body
                    .to_text()
                    .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                    .and_then(|value| value.get("status").and_then(serde_json::Value::as_str).map(str::to_owned))
                    .as_deref()
                    == Some("escalate")
                    && let Some(run) = &mut run
                {
                    run.terminal = true;
                }
            }
            _ => {}
        }
    }
    run
}
