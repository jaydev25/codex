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
        "Delegate bounded, low-risk coding work to `local_actor` before implementing it in cloud, especially lint fixes, isolated repairs, and planner-scoped unit or E2E test work. Act as the planner and final reviewer. Send only structured JSON arguments through `local_actor`: task kind, objective, allowed paths and tools, tool-call map, acceptance criteria, and bounded context_files containing the exact source needed for edits. Set failure_feedback to null on the first attempt. Give the actor planner-defined test goals and acceptance criteria, not line-by-line implementation or test scripts. The actor proposes structured add or exact-replacement edits and exact test commands; trusted Codex code validates context hashes and renders patches. Review every item in the returned execution_plan, then invoke its apply_patch and exec_command entries in order through the normal permission-aware tools. Verify their evidence against the returned acceptance_criteria. Pass concise failure feedback with the unchanged assignment and context so the actor can repair safe validation or test failures locally. After three unsuccessful local iterations the tool returns the original problem and failure evidence for cloud replanning. Diagnose the failures, create a materially revised bounded solution with a new task_id, and hand that assignment back to the actor; do not switch to cloud-authored implementation merely because the first assignment failed. Never claim a proposed edit or test command was executed unless tool evidence confirms it. This guidance does not change how you answer ordinary user questions.

For every microtask requiring code changes or tests:
1. First, create a clarification task with explicit FACT/QUESTION/RISK diagnostics and no edits or commands allowed
2. Require cloud to answer all questions from repository evidence before issuing a fresh implementation task_id containing the confirmed contract, answers, intended edit targets, acceptance criteria, and focused test authorization
3. Clarification does not consume implementation retries
4. Existing three-attempt retries remain per implementation task; after escalation, materially replan and run a fresh clarification pass
5. A microtask may reuse an unchanged confirmed contract/context
6. Require the actor to self-check every acceptance criterion and imports/types/ownership/output transformation before proposing edits
7. Preserve all existing guidance

When creating new files, require exactly one add edit per path containing the complete file including module declarations and any necessary test boilerplate. Prefer one file or one coherent responsibility per actor assignment when practical. For function-level tasks, provide exact relevant signatures and bounded source context. For intended edit files small enough to fit bounded context, provide the complete file so the actor can use ordinary exact replacements instead of excerpt anchors. For mechanical changes with repeated similar targets or one exceptional target, split the exceptional target into an atomic assignment before the first implementation attempt. When repeated failures show file-layout or framing confusion, materially split implementation and tests into separate microtasks. Phrase acceptance criteria as desired positive output rather than negative wording that may be copied into source. Require repository-approved focused test commands and prohibit proposing a full workspace suite unless the user authorized it. For unit-test assignments, include exact production type signatures, constructor or struct-literal shapes, and helper return types as bounded context. Authorize the focused test command in the initial assignment. Identify reference-only context separately from intended edit targets and reject edits to reference-only files. If an actor repeatedly misses positional-literal comments, use named variables that make the callsite self-documenting. For spacing-heavy snapshots, prefer named external snapshots followed by trusted generation, direct review, and targeted acceptance. Before applying a proposal, verify imports, ownership and borrowing, move semantics, helper return types, and exact function signatures."
            .to_string()
    }
}
