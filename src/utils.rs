use std::time::Duration;

use ratatui::layout::{
    Constraint,
    Direction,
    Layout,
    Rect,
};

pub fn format_duration(value: &Duration) -> String {
    if value.as_secs() < 60 * 60 {
        format!(
            "{:0>2}:{:0>2}.{:0>2}",
            value.as_secs() / 60,
            value.as_secs() % 60,
            value.subsec_millis() / 10
        )
    } else {
        format!(
            "{:0>2}:{:0>2}:{:0>2}",
            value.as_secs() / (60 * 60),
            (value.as_secs() / 60) % 60,
            value.as_secs() % 60
        )
    }
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

const SIZE_1KB: u64 = 1024;
const SIZE_1MB: u64 = 1024 * SIZE_1KB;
const SIZE_1GB: u64 = 1024 * SIZE_1MB;

pub fn format_file_size(size: u64) -> String {
    if size >= SIZE_1GB * 2 {
        format!("{:.2} GB", (size as f64) / (SIZE_1GB as f64))
    } else if size >= SIZE_1MB * 2 {
        format!("{:.2} MB", (size as f64) / (SIZE_1MB as f64))
    } else if size >= SIZE_1KB * 2 {
        format!("{:.2} KB", (size as f64) / (SIZE_1KB as f64))
    } else {
        format!("{} bytes", size)
    }
}
