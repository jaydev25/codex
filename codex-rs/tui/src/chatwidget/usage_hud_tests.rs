use super::format_usage_hud;

#[test]
fn complete_state() {
    let result = format_usage_hud(Some(74.0), Some(81.0), Some("3:45 PM"), 1_200_000);
    insta::assert_snapshot!(result, @r###"5h 74% · W 81% · Next reset 3:45 PM · Saved ~1.2M"###);
}

#[test]
fn zero_state() {
    let result = format_usage_hud(None, None, None, 0);
    insta::assert_snapshot!(result, @r###"Saved ~0"###);
}
