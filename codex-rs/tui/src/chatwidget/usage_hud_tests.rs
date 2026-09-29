use super::UsageHudContext;
use super::UsageHudData;
use super::format_usage_hud;
use ratatui::text::Text;

fn render_hud(hud: &Text<'_>) -> String {
    hud.lines
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn complete_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: Some(74.0),
        weekly: Some(81.0),
        five_hour_reset: Some("15:45"),
        weekly_reset: Some("09:30 on 5 Oct"),
        context: Some(UsageHudContext {
            used_tokens: 24_000,
            limit_tokens: 272_000,
        }),
        analysis_raw_tokens: 1_500_000,
        analysis_forwarded_tokens: 300_000,
        analysis_saved_tokens: 1_200_000,
        actor_calls: 2,
        actor_prompt_tokens: 2_700,
        actor_generated_tokens: 300,
    });
    insta::assert_snapshot!(render_hud(&result), @r###"
    5h 74% (resets 15:45) · W 81% (resets 09:30 on 5 Oct) · Context 24K/272K
    Coordinator 2 · 2.7K/300 local · Cloud 1.5M→300K · 1.2M saved
    "###);
}

#[test]
fn zero_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: None,
        weekly: None,
        five_hour_reset: None,
        weekly_reset: None,
        context: None,
        analysis_raw_tokens: 0,
        analysis_forwarded_tokens: 0,
        analysis_saved_tokens: 0,
        actor_calls: 0,
        actor_prompt_tokens: 0,
        actor_generated_tokens: 0,
    });
    insta::assert_snapshot!(render_hud(&result), @r###"

    Cloud 0→0 · 0 saved
    "###);
}

#[test]
fn actor_only_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: None,
        weekly: None,
        five_hour_reset: None,
        weekly_reset: None,
        context: None,
        analysis_raw_tokens: 0,
        analysis_forwarded_tokens: 0,
        analysis_saved_tokens: 0,
        actor_calls: 1,
        actor_prompt_tokens: 5_000,
        actor_generated_tokens: 2_345,
    });
    insta::assert_snapshot!(render_hud(&result), @r###"

    Coordinator 1 · 5K/2.35K local · Cloud 0→0 · 0 saved
    "###);
}

#[test]
fn context_only_state() {
    let result = format_usage_hud(UsageHudData {
        five_hour: None,
        weekly: None,
        five_hour_reset: None,
        weekly_reset: None,
        context: Some(UsageHudContext {
            used_tokens: 0,
            limit_tokens: 32_768,
        }),
        analysis_raw_tokens: 0,
        analysis_forwarded_tokens: 0,
        analysis_saved_tokens: 0,
        actor_calls: 0,
        actor_prompt_tokens: 0,
        actor_generated_tokens: 0,
    });
    insta::assert_snapshot!(render_hud(&result), @r###"
    Context 0/32.8K
    Cloud 0→0 · 0 saved
    "###);
}
