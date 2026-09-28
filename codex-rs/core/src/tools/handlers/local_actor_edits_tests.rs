use super::*;
use pretty_assertions::assert_eq;
use std::fs;

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
        render_actor_edits(&edits, &[context], None).unwrap(),
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
        render_actor_edits(&[edit], &[context], None)
            .unwrap_err()
            .contains("exactly once")
    );
}

fn truncated_context() -> ActorContextFile {
    ActorContextFile {
        path: "src/lib.rs".to_string(),
        content: "    run(old);\n".to_string(),
        truncated: true,
    }
}

fn excerpt_edit(context: &ActorContextFile) -> ActorEdit {
    ActorEdit::ReplaceExcerpt {
        path: context.path.clone(),
        context_sha256: context.content_sha256(),
        before_anchor: "run(".to_string(),
        old_text: "old".to_string(),
        after_anchor: ");".to_string(),
        new_text: "new".to_string(),
    }
}

#[test]
fn renders_truncated_excerpt_against_unique_live_source() {
    let temp = tempfile::tempdir().unwrap();
    let src = temp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.rs"), "fn main() {\n    run(old);\n}\n").unwrap();
    let context = truncated_context();

    assert_eq!(
        render_actor_edits(
            &[excerpt_edit(&context)],
            &[context],
            Some(temp.path()),
        )
        .unwrap(),
        vec![
            "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-    run(old);\n+    run(new);\n*** End Patch"
                .to_string()
        ]
    );
}

#[test]
fn rejects_stale_and_ambiguous_live_excerpt_targets() {
    let temp = tempfile::tempdir().unwrap();
    let src = temp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let context = truncated_context();
    let edit = excerpt_edit(&context);

    fs::write(src.join("lib.rs"), "    run(changed);\n").unwrap();
    assert!(
        render_actor_edits(
            std::slice::from_ref(&edit),
            std::slice::from_ref(&context),
            Some(temp.path())
        )
        .unwrap_err()
        .contains("stale")
    );

    fs::write(src.join("lib.rs"), "    run(old);\n    run(old);\n").unwrap();
    assert!(
        render_actor_edits(&[edit], &[context], Some(temp.path()))
            .unwrap_err()
            .contains("ambiguous")
    );
}

#[test]
fn rejects_excerpt_path_that_escapes_working_directory() {
    let temp = tempfile::tempdir().unwrap();
    let context = ActorContextFile {
        path: "../src/lib.rs".to_string(),
        content: "run(old);".to_string(),
        truncated: true,
    };
    let edit = excerpt_edit(&context);

    assert!(
        render_actor_edits(&[edit], &[context], Some(temp.path()))
            .unwrap_err()
            .contains("escapes")
    );
}
