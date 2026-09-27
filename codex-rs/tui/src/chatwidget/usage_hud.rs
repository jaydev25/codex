use crate::status::format_tokens_compact;

use ratatui::style::Stylize;
use ratatui::text::Line;

pub(crate) fn format_usage_hud(
    five_hour: Option<f64>,
    weekly: Option<f64>,
    next_reset: Option<&str>,
    saved_tokens: i64,
) -> Line<'static> {
    let mut parts = Vec::new();

    if let Some(percent) = five_hour {
        parts.push(format!("5h {percent:.0}%"));
    }

    if let Some(percent) = weekly {
        parts.push(format!("W {percent:.0}%"));
    }

    if let Some(value) = next_reset {
        parts.push(format!("Next reset {value}"));
    }

    parts.push(format!("Saved ~{}", format_tokens_compact(saved_tokens)));

    let text = parts.join(" · ");
    text.dim().into()
}

#[cfg(test)]
#[path = "usage_hud_tests.rs"]
mod tests;
