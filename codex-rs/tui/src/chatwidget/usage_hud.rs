use crate::status::format_tokens_compact;

use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Text;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UsageHudContext {
    pub(crate) used_tokens: i64,
    pub(crate) limit_tokens: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UsageHudData<'a> {
    pub(crate) five_hour: Option<f64>,
    pub(crate) weekly: Option<f64>,
    pub(crate) five_hour_reset: Option<&'a str>,
    pub(crate) weekly_reset: Option<&'a str>,
    pub(crate) context: Option<UsageHudContext>,
    pub(crate) analysis_raw_tokens: u64,
    pub(crate) analysis_forwarded_tokens: u64,
    pub(crate) analysis_saved_tokens: u64,
    pub(crate) actor_calls: u64,
    pub(crate) actor_prompt_tokens: u64,
    pub(crate) actor_generated_tokens: u64,
}

pub(crate) fn format_usage_hud(data: UsageHudData<'_>) -> Text<'static> {
    let mut summary_parts = Vec::new();

    if let Some(percent) = data.five_hour {
        let reset = data
            .five_hour_reset
            .map(|value| format!(" (resets {value})"))
            .unwrap_or_default();
        summary_parts.push(format!("5h {percent:.0}%{reset}"));
    }

    if let Some(percent) = data.weekly {
        let reset = data
            .weekly_reset
            .map(|value| format!(" (resets {value})"))
            .unwrap_or_default();
        summary_parts.push(format!("W {percent:.0}%{reset}"));
    }

    if let Some(context) = data.context {
        let used = context.used_tokens.max(0);
        summary_parts.push(format!(
            "Context {}{}{}",
            format_tokens_compact(used),
            '/',
            format_tokens_compact(context.limit_tokens)
        ));
    }

    let mut stats_parts = Vec::new();
    if data.actor_calls > 0 {
        let prompt_tokens = data.actor_prompt_tokens.min(i64::MAX as u64) as i64;
        let generated_tokens = data.actor_generated_tokens.min(i64::MAX as u64) as i64;
        stats_parts.push(format!("Coordinator {}", data.actor_calls));
        stats_parts.push(format!(
            "{}/{} local",
            format_tokens_compact(prompt_tokens),
            format_tokens_compact(generated_tokens)
        ));
    }

    let analysis_raw = data.analysis_raw_tokens.min(i64::MAX as u64) as i64;
    let analysis_forwarded = data.analysis_forwarded_tokens.min(i64::MAX as u64) as i64;
    let analysis_saved = data.analysis_saved_tokens.min(i64::MAX as u64) as i64;
    stats_parts.push(format!(
        "Cloud {}→{}",
        format_tokens_compact(analysis_raw),
        format_tokens_compact(analysis_forwarded),
    ));
    stats_parts.push(format!("{} saved", format_tokens_compact(analysis_saved)));

    Text::from(vec![
        Line::from(summary_parts.join(" · ")).dim(),
        Line::from(stats_parts.join(" · ")).dim(),
    ])
}

#[cfg(test)]
#[path = "usage_hud_tests.rs"]
mod tests;
