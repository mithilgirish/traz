use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Render git diff patch string into a vector of colored ratatui Lines.
pub fn render_diff(patch: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for line in patch.lines() {
        let style = if line.starts_with('+') && !line.starts_with("+++") {
            Style::default().fg(Color::Green)
        } else if line.starts_with('-') && !line.starts_with("---") {
            Style::default().fg(Color::Red)
        } else if line.starts_with("@@") {
            Style::default().fg(Color::Cyan)
        } else if line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("---")
            || line.starts_with("+++")
        {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        lines.push(Line::from(Span::styled(line.to_string(), style)));
    }
    lines
}
