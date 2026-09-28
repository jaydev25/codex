use super::UsageHudContext;
use super::UsageHudData;
use super::format_usage_hud;

#[test]
fn complete_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: Some(74.0),
        weekly: Some(81.0),
        next_reset: Some("3:45 PM"),
        context: Some(UsageHudContext {
            used_tokens: 24_000,
            limit_tokens: 272_000,
        }),
        analysis_saved_tokens: 1_200_000,
        actor_calls: 2,
        actor_prompt_tokens: 2_700,
        actor_generated_tokens: 300,
    });
    insta::assert_snapshot!(result, @r###"5h 74% · W 81% · Next reset 3:45 PM · Context 24K/272K · Actor local 2 calls · 2.7K in · 300 generated · Cloud saved ~1.2M"###);
}

#[test]
fn zero_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: None,
        weekly: None,
        next_reset: None,
        context: None,
        analysis_saved_tokens: 0,
        actor_calls: 0,
        actor_prompt_tokens: 0,
        actor_generated_tokens: 0,
    });
    insta::assert_snapshot!(result, @r###"Cloud saved ~0"###);
}

#[test]
fn actor_only_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: None,
        weekly: None,
        next_reset: None,
        context: None,
        analysis_saved_tokens: 0,
        actor_calls: 1,
        actor_prompt_tokens: 5_000,
        actor_generated_tokens: 2_345,
    });
    insta::assert_snapshot!(result, @r###"Actor local 1 calls · 5K in · 2.35K generated · Cloud saved ~0"###);
}

#[test]
fn context_only_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: None,
        weekly: None,
        next_reset: None,
        context: Some(UsageHudContext {
            used_tokens: 0,
            limit_tokens: 32_768,
        }),
        analysis_saved_tokens: 0,
        actor_calls: 0,
        actor_prompt_tokens: 0,
        actor_generated_tokens: 0,
    });
    insta::assert_snapshot!(result, @r###"Context 0/32.8K · Cloud saved ~0"###);
}
