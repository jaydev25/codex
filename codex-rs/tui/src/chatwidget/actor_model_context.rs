pub(crate) fn parse_actor_context_length(
    input: &str,
    max_context_length: Option<u32>,
) -> Result<u32, String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err("Context length must be a positive integer.".to_string());
    }

    let value = trimmed
        .parse::<u32>()
        .map_err(|_| "Context length must be a positive integer.".to_string())?;

    if value == 0 {
        return Err("Context length must be greater than zero.".to_string());
    }

    if let Some(max) = max_context_length
        && value > max
    {
        return Err(format!(
            "Context length {value} exceeds this model's maximum of {max}."
        ));
    }

    Ok(value)
}

#[cfg(test)]
#[path = "actor_model_context_tests.rs"]
mod tests;
