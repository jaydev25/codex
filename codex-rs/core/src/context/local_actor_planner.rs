use super::ContextualUserFragment;
use codex_protocol::models::ContentItemKind;

/// Division-of-labor guidance when a local coordination model is configured.
pub(crate) struct LocalCoordinatorGuidance;

impl LocalCoordinatorGuidance {
    pub(crate) fn matches_text(text: &str) -> bool {
        text.contains("<local_coordinator_guidance>") || text.contains("<local_actor_planner>")
    }
}

impl ContextualUserFragment for LocalCoordinatorGuidance {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("local_coordinator.guidance".to_string())
    }

    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        (
            "<local_coordinator_guidance>",
            "</local_coordinator_guidance>",
        )
    }

    fn body(&self) -> String {
        "For nontrivial coding tasks, call `local_coordinator` before editing with the user's objective, constraints, and bounded repository context. The local model owns decomposition, repetitive inspection, selection of focused verification commands, and triage of test evidence. It must assign implementation and test authoring to you, the cloud worker, and it must never write source code, patches, or tests. Execute local-owned inspect and verify commands only through the normal permission-aware tools. You own architecture decisions, every file edit, test authoring, and final review. After implementation, run the coordinator-selected focused checks; when output is nontrivial, let local analysis reduce it to cited evidence before it enters cloud context, then call `local_coordinator` in verify phase with that bounded evidence. Treat coordination as advisory and independently verify behavior. Only locally analyzed raw-versus-forwarded command output counts as cloud input saved; local-model prompt or completion tokens are usage, not savings. If the coordinator is unavailable or invalid, continue in cloud and state the fallback in commentary. Skip coordination for ordinary questions and trivial changes."
            .to_string()
    }
}
