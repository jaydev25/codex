use super::*;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn rendered(buffer: &Buffer) -> String {
    let width = usize::from(buffer.area.width);
    let rows: Vec<String> = buffer
        .content
        .chunks(width)
        .map(|chunk| {
            chunk
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();

    let mut trimmed_rows = rows.to_vec();
    while let Some(last) = trimmed_rows.last() {
        if last.is_empty() {
            trimmed_rows.pop();
        } else {
            break;
        }
    }

    trimmed_rows.join("\n")
}

#[test]
fn background_command_lines_respects_visible_limit() {
    let recent_output = vec![
        "output one".to_string(),
        "output two".to_string(),
        "output three".to_string(),
    ];
    let commands = [BackgroundCommandDisplay {
        command: "cargo test",
        recent_output: &recent_output,
    }];
    let width = 40;
    let max_lines = 3;
    let actual: Vec<String> = background_command_lines(&commands, width, max_lines)
        .into_iter()
        .map(|line| line.to_string())
        .collect();
    let expected = vec![
        "cargo test".to_string(),
        "output one".to_string(),
        "output two".to_string(),
    ];
    assert_eq!(actual, expected);
}

#[test]
fn background_command_panel_renders_top_right_quarter() {
    let first_output = vec!["first command output".to_string()];
    let second_output = vec!["second command output".to_string()];

    let commands = [
        BackgroundCommandDisplay {
            command: "first command",
            recent_output: &first_output,
        },
        BackgroundCommandDisplay {
            command: "second command",
            recent_output: &second_output,
        },
    ];

    let area = Rect::new(0, 0, 80, 20);
    let mut buffer = Buffer::empty(area);
    render_background_command_panel(area, &mut buffer, &commands);

    assert_eq!(
        background_command_panel_area(area),
        Some(Rect::new(40, 0, 40, 10))
    );

    insta::assert_snapshot!(
        "background_command_panel_top_right_quarter",
        rendered(&buffer)
    );
}

#[test]
fn background_command_panel_hides_on_small_terminal() {
    let output = vec!["some command output".to_string()];
    let commands = [BackgroundCommandDisplay {
        command: "some command",
        recent_output: &output,
    }];

    let area = Rect::new(0, 0, 47, 9);
    let mut buffer = Buffer::empty(area);
    render_background_command_panel(area, &mut buffer, &commands);

    assert_eq!(background_command_panel_area(area), None);

    insta::assert_snapshot!(
        "background_command_panel_hidden_on_small_terminal",
        rendered(&buffer)
    );
}
