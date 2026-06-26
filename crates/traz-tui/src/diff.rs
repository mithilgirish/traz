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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn test_render_diff_empty() {
        let lines = render_diff("");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_render_diff_coloring() {
        let patch = "\
diff --git a/src/lib.rs b/src/lib.rs
index 854d8f9..d25b578 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,5 +1,6 @@
 unchanged line
-removed line
+added line
";
        let lines = render_diff(patch);
        assert_eq!(lines.len(), 8);

        // diff line -> Yellow
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Yellow));
        assert_eq!(lines[0].spans[0].content, "diff --git a/src/lib.rs b/src/lib.rs");

        // index line -> Yellow
        assert_eq!(lines[1].spans[0].style.fg, Some(Color::Yellow));

        // --- line -> Yellow
        assert_eq!(lines[2].spans[0].style.fg, Some(Color::Yellow));

        // +++ line -> Yellow
        assert_eq!(lines[3].spans[0].style.fg, Some(Color::Yellow));

        // @@ line -> Cyan
        assert_eq!(lines[4].spans[0].style.fg, Some(Color::Cyan));

        // unchanged line -> default style (no fg color)
        assert_eq!(lines[5].spans[0].style.fg, None);

        // removed line -> Red
        assert_eq!(lines[6].spans[0].style.fg, Some(Color::Red));

        // added line -> Green
        assert_eq!(lines[7].spans[0].style.fg, Some(Color::Green));
    }
}

