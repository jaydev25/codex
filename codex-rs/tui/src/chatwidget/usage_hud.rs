use crate::status::format_tokens_compact;

use ratatui::style::Stylize;
use ratatui::text::Line;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UsageHudContext {
    pub(crate) used_tokens: i64,
    pub(crate) limit_tokens: i64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UsageHudData<'a> {
    pub(crate) five_hour: Option<f64>,
    pub(crate) weekly: Option<f64>,
    pub(crate) next_reset: Option<&'a str>,
    pub(crate) context: Option<UsageHudContext>,
    pub(crate) analysis_saved_tokens: u64,
    pub(crate) actor_calls: u64,
    pub(crate) actor_prompt_tokens: u64,
    pub(crate) actor_generated_tokens: u64,
}

pub(crate) fn format_usage_hud(data: UsageHudData<'_>) -> Line<'static> {
    let mut parts = Vec::new();

    if let Some(percent) = data.five_hour {
        parts.push(format!("5h {percent:.0}%"));
    }

    if let Some(percent) = data.weekly {
        parts.push(format!("W {percent:.0}%"));
    }

    if let Some(value) = data.next_reset {
        parts.push(format!("Next reset {value}"));
    }

    if let Some(context) = data.context {
        let used = context.used_tokens.max(0);
        parts.push(format!(
            "Context {}{}{}",
            format_tokens_compact(used),
            '/',
            format_tokens_compact(context.limit_tokens)
        ));
    }

    if data.actor_calls > 0 {
        let prompt_tokens = data.actor_prompt_tokens.min(i64::MAX as u64) as i64;
        let generated_tokens = data.actor_generated_tokens.min(i64::MAX as u64) as i64;
        parts.push(format!("Actor local {} calls", data.actor_calls));
        parts.push(format!("{} in", format_tokens_compact(prompt_tokens)));
        parts.push(format!(
            "{} generated",
            format_tokens_compact(generated_tokens)
        ));
    }

    let analysis_saved = data.analysis_saved_tokens.min(i64::MAX as u64) as i64;
    parts.push(format!(
        "Cloud saved ~{}",
        format_tokens_compact(analysis_saved)
    ));

    let text = parts.join(" · ");
    text.dim().into()
}

#[cfg(test)]
#[path = "usage_hud_tests.rs"]
mod tests;
