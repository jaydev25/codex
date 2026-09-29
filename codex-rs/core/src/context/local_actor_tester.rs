use super::ContextualUserFragment;
use codex_protocol::models::ContentItemKind;

/// Cloud-developer and local-tester guidance when a local actor is configured.
pub(crate) struct LocalActorTester;

impl LocalActorTester {
    pub(crate) fn matches_text(text: &str) -> bool {
        text.contains("<local_actor_tester>") || text.contains("<local_actor_planner>")
    }
}

impl ContextualUserFragment for LocalActorTester {
    fn content_kind(&self) -> ContentItemKind {
        ContentItemKind("local_actor.tester".to_string())
    }

    fn role(&self) -> &'static str {
        "developer"
    }

    fn markers(&self) -> (&'static str, &'static str) {
        Self::type_markers()
    }

    fn type_markers() -> (&'static str, &'static str) {
        ("<local_actor_tester>", "</local_actor_tester>")
    }

    fn body(&self) -> String {
        "Act as the cloud developer and final reviewer. Inspect the repository, make architecture decisions, write production code, and add the baseline tests yourself. Do not delegate implementation planning or ordinary production edits to `local_actor`.

After you have a coherent implementation and baseline tests, delegate a bounded independent test/review assignment to `local_actor`. Supply the requirement, relevant complete source or diff context, approved test paths and commands, and explicit acceptance criteria. The actor has no ambient filesystem or shell authority: it may only propose additional focused tests, exact test commands for the trusted runner, diagnostics, and—after concrete failure feedback—one narrowly bounded repair when the cause is local and unambiguous. Review every returned execution-plan entry before applying or executing it through the normal permission-aware tools.

When the user explicitly asks to monitor, wait, or perform a later action after a running operation completes, delegate the phase gate to the actor as a monitor task. Use trusted polling and execution tools on its behalf, return each bounded result for evaluation, and let it advance authorized non-development phases such as testing, installation, or restart when the stated success condition is met. Unchanged running state is expected and is not a repair failure. If the next phase requires source changes, return the tester's evidence to the cloud developer instead of asking the actor to implement them. Monitoring does not broaden the user's authorization.

Set failure_feedback to null for the initial tester pass. If an actor-proposed test or command fails, return concise evidence with the unchanged assignment and context for one repair attempt. If that repair fails, the tool returns the original assignment and bounded evidence with `cloud_developer_handoff`; diagnose and fix the issue in cloud. Do not repeatedly reframe the same task for the actor, and do not require clarification calls before cloud implementation.

Keep structured actor-edit validation strict. Limit actor-writable paths to dedicated tests unless a bounded repair is explicitly authorized. Provide complete intended edit files when they fit the context limit; identify reference-only files and reject edits to them. Authorize only repository-approved focused commands, never a full workspace suite unless the user approved it. Verify actor-proposed imports, types, ownership, assertions, snapshots, commands, and claimed evidence. Never claim a proposal or command ran without tool evidence. This guidance does not change how you answer ordinary user questions."
            .to_string()
    }
}
