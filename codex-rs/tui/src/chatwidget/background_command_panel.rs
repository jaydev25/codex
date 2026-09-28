//!
//! Pure formatting module for background command display data.
//!
//! Converts borrowed background-command display data into at most max_lines
//! width-truncated Ratatui lines. Geometry and widget rendering are outside this task.

use crate::live_wrap::take_prefix_by_width;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;

/// Display data for a background command.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BackgroundCommandDisplay<'a> {
    /// The command string.
    pub(crate) command: &'a str,
    /// Recent output lines from the command.
    pub(crate) recent_output: &'a [String],
}

/// Formats background commands into Ratatui lines with width and line limits.
pub(crate) fn background_command_lines(
    commands: &[BackgroundCommandDisplay<'_>],
    width: u16,
    max_lines: usize,
) -> Vec<Line<'static>> {
    if width == 0 || max_lines == 0 {
        return vec![];
    }

    let mut lines = Vec::new();

    for command in commands {
        // Check if we've reached the maximum line limit before adding new content.
        if lines.len() >= max_lines {
            break;
        }

        // Format and add the command line (bold).
        let (text, _, _) = take_prefix_by_width(command.command, usize::from(width));
        lines.push(Line::from(text).bold());

        // Add recent output lines (dimmed), respecting max_lines.
        for output_line in command.recent_output.iter() {
            if lines.len() >= max_lines {
                break;
            }
            let (text, _, _) = take_prefix_by_width(output_line, usize::from(width));
            lines.push(Line::from(text).dim());
        }
    }

    lines
}

/// Returns the area for the background command panel if it can fit.
///
/// Returns `None` if the area is less than 48x10.
pub(crate) fn background_command_panel_area(area: Rect) -> Option<Rect> {
    if area.width < 48 || area.height < 10 {
        return None;
    }

    let width = area.width / 2;
    let height = area.height / 2;
    let x = area.x + area.width - width;
    let y = area.y;

    Some(Rect::new(x, y, width, height))
}

/// Renders the background command panel into the given buffer.
pub(crate) fn render_background_command_panel(
    area: Rect,
    buf: &mut Buffer,
    commands: &[BackgroundCommandDisplay<'_>],
) {
    if commands.is_empty() {
        return;
    }

    let Some(panel_area) = background_command_panel_area(area) else {
        return;
    };

    Clear.render(panel_area, buf);

    let count = commands.len();
    let title = format!("Background commands ({count})").bold();
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(panel_area);

    let lines = background_command_lines(commands, inner.width, usize::from(inner.height));
    let paragraph = Paragraph::new(lines).block(block);
    paragraph.render(panel_area, buf);
}

#[cfg(test)]
#[path = "background_command_panel_tests.rs"]
mod tests;
