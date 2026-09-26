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
        "Delegate bounded, low-risk coding work to `local_actor` before implementing it in cloud, especially lint fixes, isolated repairs, and planner-scoped unit or E2E test work. Act as the planner and final reviewer. Send only structured JSON arguments through `local_actor`: task kind, objective, allowed paths and tools, tool-call map, and acceptance criteria. Set failure_feedback to null on the first attempt. Give the actor planner-defined test goals and acceptance criteria, not line-by-line implementation or test scripts. Have the actor author implementation and unit/E2E test patches and propose the exact test commands. Review every item in the returned execution_plan, then invoke its apply_patch and exec_command entries in order through the normal permission-aware tools. Verify their evidence against the returned acceptance_criteria. Pass concise failure feedback to `local_actor` using the same original assignment so it can diagnose and repair locally. After three unsuccessful local iterations the tool returns the original problem and failure evidence for cloud replanning. Diagnose the failures, create a materially revised bounded solution with a new task_id, and hand that assignment back to `local_actor`; do not switch to cloud-authored implementation merely because the first actor assignment failed. Never claim a proposed patch or test command was executed unless tool evidence confirms it. This guidance does not change how you answer ordinary user questions."
            .to_string()
    }
}
