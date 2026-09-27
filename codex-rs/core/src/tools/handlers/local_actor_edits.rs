//! Trusted rendering for validated local-actor structured edits.

use codex_apply_patch::Hunk;
use codex_local_models::ActorContextFile;
use codex_local_models::ActorEdit;

pub(super) fn render_actor_edits(
    edits: &[ActorEdit],
    context_files: &[ActorContextFile],
) -> Result<Vec<String>, String> {
    edits
        .iter()
        .map(|edit| {
            let (path, patch) = match edit {
                ActorEdit::Add { path, content } => (
                    path.as_str(),
                    format!(
                        "*** Begin Patch\n*** Add File: {path}\n{}*** End Patch",
                        prefixed_lines(content, '+')
                    ),
                ),
                ActorEdit::Replace {
                    path,
                    context_sha256,
                    old_text,
                    new_text,
                } => {
                    let Some(context) = context_files.iter().find(|context| context.path == *path)
                    else {
                        return Err(format!("replacement has no context for {path}"));
                    };
                    if context.truncated || context.content_sha256() != *context_sha256 {
                        return Err(format!(
                            "replacement context is stale or truncated for {path}"
                        ));
                    }
                    if old_text.is_empty() || context.content.match_indices(old_text).count() != 1 {
                        return Err(format!("old_text must occur exactly once in {path}"));
                    }
                    let (old_lines, new_lines) =
                        replacement_line_block(&context.content, old_text, new_text)?;
                    (
                        path.as_str(),
                        format!(
                            "*** Begin Patch\n*** Update File: {path}\n@@\n{}{}*** End Patch",
                            prefixed_lines(old_lines, '-'),
                            prefixed_lines(&new_lines, '+')
                        ),
                    )
                }
            };
            validate_rendered_patch(path, &patch)?;
            Ok(patch)
        })
        .collect()
}

fn replacement_line_block<'a>(
    content: &'a str,
    old_text: &str,
    new_text: &str,
) -> Result<(&'a str, String), String> {
    let Some(match_start) = content.find(old_text) else {
        return Err("replacement text is absent from context".to_string());
    };
    let line_start = content[..match_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let match_end = match_start + old_text.len();
    let line_end = content[match_end..]
        .find('\n')
        .map_or(content.len(), |index| match_end + index + 1);
    let old_lines = &content[line_start..line_end];
    Ok((old_lines, old_lines.replacen(old_text, new_text, 1)))
}

fn prefixed_lines(text: &str, prefix: char) -> String {
    let mut prefixed = String::new();
    for line in text.split_inclusive('\n') {
        prefixed.push(prefix);
        prefixed.push_str(line);
        if !line.ends_with('\n') {
            prefixed.push('\n');
        }
    }
    prefixed
}

fn validate_rendered_patch(path: &str, patch: &str) -> Result<(), String> {
    let parsed = codex_apply_patch::parse_patch(patch).map_err(|error| {
        format!("trusted actor edit renderer produced an invalid patch: {error}")
    })?;
    if parsed.hunks.len() != 1
        || parsed.hunks[0].path().to_string_lossy() != path
        || matches!(
            &parsed.hunks[0],
            Hunk::UpdateFile {
                move_path: Some(_),
                ..
            }
        )
    {
        return Err("trusted actor edit renderer changed the declared path".to_string());
    }
    Ok(())
}

#[cfg(test)]
#[path = "local_actor_edits_tests.rs"]
mod tests;
