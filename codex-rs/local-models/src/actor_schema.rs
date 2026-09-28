use serde_json::Value;

/// Represents the kind of actor edit operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ActorEditKind {
    Add,
    Replace,
    ReplaceExcerpt,
}

pub(super) fn restrict_actor_edit_schema(
    schema: &mut Value,
    target_kind: ActorEditKind,
) -> Result<(), String> {
    let definitions_key = if schema.get("definitions").is_some() {
        "definitions"
    } else if schema.get("$defs").is_some() {
        "$defs"
    } else {
        return Err("actor result schema has no definitions".to_string());
    };
    let variants = schema
        .get_mut(definitions_key)
        .and_then(|definitions| definitions.get_mut("ActorEdit"))
        .and_then(|actor_edit| actor_edit.get_mut("oneOf"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "actor result schema has no ActorEdit variants".to_string())?;
    filter_variants_by_kind(variants, target_kind)
}

/// Retains the single schema variant for the requested actor edit kind.
pub(super) fn filter_variants_by_kind(
    variants: &mut Vec<Value>,
    target_kind: ActorEditKind,
) -> Result<(), String> {
    let target_str = match target_kind {
        ActorEditKind::Add => "add",
        ActorEditKind::Replace => "replace",
        ActorEditKind::ReplaceExcerpt => "replace_excerpt",
    };

    let mut matched_variants: Vec<Value> = variants
        .iter()
        .filter(|variant| {
            variant
                .get("properties")
                .and_then(|props| props.get("kind"))
                .is_some_and(|kind| {
                    if let Some(const_val) = kind.get("const")
                        && let Some(str_val) = const_val.as_str()
                    {
                        return str_val == target_str;
                    }

                    if let Some(enum_vals) = kind.get("enum").and_then(Value::as_array)
                        && enum_vals.len() == 1
                        && let Some(first_val) = enum_vals.first()
                        && let Some(str_val) = first_val.as_str()
                    {
                        return str_val == target_str;
                    }

                    false
                })
        })
        .cloned()
        .collect();

    if matched_variants.len() != 1 {
        return Err(format!(
            "expected exactly one matching actor edit variant, found {}",
            matched_variants.len()
        ));
    }

    let matching = matched_variants
        .pop()
        .ok_or_else(|| "missing matching actor edit variant".to_string())?;
    *variants = vec![matching];

    Ok(())
}

#[cfg(test)]
#[path = "actor_schema_tests.rs"]
mod tests;
