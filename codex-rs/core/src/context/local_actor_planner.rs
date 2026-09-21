use super::ContextualUserFragment;
use codex_protocol::models::ContentItemKind;

/// Bounded cloud-planner guidance when a local actor is configured.
pub(crate) struct LocalActorPlanner;

impl LocalActorPlanner {
    pub(crate) fn matches_text(text: &str) -> bool {
        text.contains("<local_actor_planner>")
    }
}

impl ContextualUserFragment for LocalActorPlanner {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("local_actor.planner".to_string())
    }

    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("<local_actor_planner>", "</local_actor_planner>")
    }

    fn body(&self) -> String {
        "For coding work delegated to the local actor, act as the planner and final reviewer. Send only structured JSON arguments through `local_actor`: task kind, objective, allowed paths and tools, tool-call map, and acceptance criteria. Do not put line-by-line implementation or test scripts in the assignment. Have the actor propose implementation and unit/E2E tests. Review each returned apply_patch proposal, then apply it with the normal permission-aware `apply_patch` tool and run actor-proposed tests with the normal execution tool. Pass concise failure feedback to `local_actor` using the same original assignment so it can diagnose and repair locally. After three unsuccessful local iterations the tool returns an escalation with the original problem; take it back and solve or replan with the selected cloud model. Never claim a proposed patch or test command was executed unless tool evidence confirms it. This guidance does not change how you answer ordinary user questions."
            .to_string()
    }
}
