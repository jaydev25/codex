use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn filter_variants_by_kind_const_kinds_add_marker1_replace_marker2_request_replace_ok_and_vector_replace_marker2()
 {
    let mut variants = vec![
        json!({"properties":{"kind":{"const":"add"}},"marker":1}),
        json!({"properties":{"kind":{"const":"replace"}},"marker":2}),
    ];
    let result = filter_variants_by_kind(&mut variants, ActorEditKind::Replace);
    assert_eq!(result, Ok(()));
    assert_eq!(
        variants,
        vec![json!({"properties":{"kind":{"const":"replace"}},"marker":2})]
    );
}

#[test]
fn filter_variants_by_kind_enum_kinds_add_marker1_replace_marker2_request_add_ok_and_vector_add_marker1()
 {
    let mut variants = vec![
        json!({"properties":{"kind":{"enum":["add"]}},"marker":1}),
        json!({"properties":{"kind":{"enum":["replace"]}},"marker":2}),
    ];
    let result = filter_variants_by_kind(&mut variants, ActorEditKind::Add);
    assert_eq!(result, Ok(()));
    assert_eq!(
        variants,
        vec![json!({"properties":{"kind":{"enum":["add"]}},"marker":1})]
    );
}

#[test]
fn filter_variants_by_kind_const_kinds_add_marker1_request_replace_err() {
    let mut variants = vec![json!({"properties":{"kind":{"const":"add"}},"marker":1})];
    let result = filter_variants_by_kind(&mut variants, ActorEditKind::Replace);
    assert!(result.is_err());
}

#[test]
fn filter_variants_by_kind_const_kinds_replace_marker1_replace_marker2_request_replace_err() {
    let mut variants = vec![
        json!({"properties":{"kind":{"const":"replace"}},"marker":1}),
        json!({"properties":{"kind":{"const":"replace"}},"marker":2}),
    ];
    let result = filter_variants_by_kind(&mut variants, ActorEditKind::Replace);
    assert!(result.is_err());
}
