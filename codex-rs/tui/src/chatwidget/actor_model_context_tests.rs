use super::*;
use pretty_assertions::assert_eq;

#[test]
fn test_whitespace_trimmed() {
    let result = parse_actor_context_length("  42  ", None);
    assert_eq!(result, Ok(42));
}

#[test]
fn test_non_numeric_input() {
    let result = parse_actor_context_length("abc", None);
    assert_eq!(
        result,
        Err("Context length must be a positive integer.".to_string())
    );
}

#[test]
fn test_zero_value() {
    let result = parse_actor_context_length("0", None);
    assert_eq!(
        result,
        Err("Context length must be greater than zero.".to_string())
    );
}

#[test]
fn test_maximum_boundary() {
    let result = parse_actor_context_length("100", Some(100));
    assert_eq!(result, Ok(100));
}

#[test]
fn test_above_maximum() {
    let result = parse_actor_context_length("101", Some(100));
    assert_eq!(
        result,
        Err("Context length 101 exceeds this model's maximum of 100.".to_string())
    );
}

#[test]
fn test_unknown_maximum() {
    let result = parse_actor_context_length("50", None);
    assert_eq!(result, Ok(50));
}
