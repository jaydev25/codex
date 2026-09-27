use super::*;
use pretty_assertions::assert_eq;

#[test]
fn renders_add_and_exact_replacement_as_trusted_patches() {
    let context = ActorContextFile {
        path: "src/lib.rs".to_string(),
        content: "fn old() {}\n".to_string(),
        truncated: false,
    };
    let edits = vec![
        ActorEdit::Add {
            path: "src/new.rs".to_string(),
            content: "fn added() {}\n".to_string(),
        },
        ActorEdit::Replace {
            path: context.path.clone(),
            context_sha256: context.content_sha256(),
            old_text: "old".to_string(),
            new_text: "new".to_string(),
        },
    ];

    assert_eq!(
        render_actor_edits(&edits, &[context]).unwrap(),
        vec![
            "*** Begin Patch\n*** Add File: src/new.rs\n+fn added() {}\n*** End Patch"
                .to_string(),
            "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-fn old() {}\n+fn new() {}\n*** End Patch"
                .to_string(),
        ]
    );
}

#[test]
fn rejects_stale_or_ambiguous_replacement_context() {
    let context = ActorContextFile {
        path: "src/lib.rs".to_string(),
        content: "old\nold\n".to_string(),
        truncated: false,
    };
    let edit = ActorEdit::Replace {
        path: context.path.clone(),
        context_sha256: context.content_sha256(),
        old_text: "old".to_string(),
        new_text: "new".to_string(),
    };

    assert!(
        render_actor_edits(&[edit], &[context])
            .unwrap_err()
            .contains("exactly once")
    );
}
