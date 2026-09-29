//! Read-only consultation with the configured local coordination model.

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::context::boxed_tool_output;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::CoreToolRuntime;
use crate::tools::registry::ToolExecutor;
use codex_local_models::LocalActorStatsEvent;
use codex_local_models::LocalPlannerError;
use codex_local_models::PlannerRequest;
use codex_local_models::append_local_actor_stats_event;
use codex_local_models::run_local_planner;
use codex_tools::JsonSchema;
use codex_tools::ResponsesApiTool;
use codex_tools::ToolName;
use codex_tools::ToolSpec;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Duration;
use url::Url;

const TOOL_NAME: &str = "local_coordinator";

pub(crate) struct LocalCoordinatorHandler;

impl ToolExecutor<ToolInvocation> for LocalCoordinatorHandler {
    fn tool_name(&self) -> ToolName {
        ToolName::plain(TOOL_NAME)
    }

    fn spec(&self) -> ToolSpec {
        let string = || JsonSchema::string(None);
        let context_file = JsonSchema::object(
            BTreeMap::from([
                ("path".to_string(), string()),
                ("content".to_string(), string()),
                ("truncated".to_string(), JsonSchema::boolean(None)),
            ]),
            Some(vec![
                "path".to_string(),
                "content".to_string(),
                "truncated".to_string(),
            ]),
            Some(false.into()),
        );
        ToolSpec::Function(ResponsesApiTool {
            name: TOOL_NAME.to_string(),
            description: "Delegate coordination and repetitive verification to the configured local model. It decomposes work, assigns coding and test authoring to the cloud worker, and proposes read-only inspection or focused verification commands for execution through normal permission-aware tools. Call it again in verify phase with bounded test evidence for local triage. It cannot edit files or write tests."
                .to_string(),
            strict: true,
            defer_loading: None,
            parameters: JsonSchema::object(
                BTreeMap::from([
                    ("schema_version".to_string(), JsonSchema::integer(None)),
                    (
                        "phase".to_string(),
                        JsonSchema::string_enum(vec![json!("plan"), json!("verify")], None),
                    ),
                    ("objective".to_string(), string()),
                    (
                        "constraints".to_string(),
                        JsonSchema::array(string(), None),
                    ),
                    (
                        "context_files".to_string(),
                        JsonSchema::array(context_file, None),
                    ),
                ]),
                Some(vec![
                    "schema_version".to_string(),
                    "phase".to_string(),
                    "objective".to_string(),
                    "constraints".to_string(),
                    "context_files".to_string(),
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
                    "local_coordinator requires structured function arguments".to_string(),
                ));
            };
            let request: PlannerRequest = parse_arguments(arguments)?;
            let policy = &invocation.turn.config.local_analysis;
            let (Some(base_url), Some(backend_model)) = (
                policy.backend_url.as_deref(),
                policy.backend_model.as_deref(),
            ) else {
                return Err(FunctionCallError::RespondToModel(
                    "local coordinator backend is not configured".to_string(),
                ));
            };
            let base_url = Url::parse(base_url).map_err(|error| {
                FunctionCallError::RespondToModel(format!("invalid local coordinator URL: {error}"))
            })?;
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(180))
                .build()
                .map_err(|error| FunctionCallError::RespondToModel(error.to_string()))?;
            let output = run_local_planner(&client, &base_url, backend_model, &request)
                .await
                .map_err(|error| match error {
                    LocalPlannerError::InvalidRequest(message)
                    | LocalPlannerError::InvalidResponse(message) => {
                        FunctionCallError::RespondToModel(message)
                    }
                    LocalPlannerError::Transport(error) => {
                        FunctionCallError::RespondToModel(format!(
                            "local coordinator is unavailable; continue with cloud coordination: {error}"
                        ))
                    }
                })?;
            if let Some(usage) = output.usage
                && let Err(error) = append_local_actor_stats_event(
                    &invocation.turn.config.codex_home,
                    &LocalActorStatsEvent {
                        prompt_tokens: usage.prompt_tokens,
                        completion_tokens: usage.completion_tokens,
                        total_tokens: usage.total_tokens,
                    },
                )
            {
                tracing::warn!(%error, "failed to record local coordinator usage");
            }
            let response = json!({
                "status": if output.result.needs_more_context { "needs_more_context" } else { "coordinated" },
                "coordination": output.result,
                "next_action": "execute_local_steps_or_delegate_cloud_owned_code_work",
            });
            Ok(boxed_tool_output(FunctionToolOutput::from_text(
                response.to_string(),
                Some(true),
            )))
        })
    }
}

impl CoreToolRuntime for LocalCoordinatorHandler {}
