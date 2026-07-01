use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Row, Table, TableState, Wrap};

use crate::app::{App, AppMode, ConfirmAction};
use crate::diff::render_diff;

// Dynamic Developer Color Theme (Catppuccin Mocha & Latte inspired)
pub struct Theme {
    pub bg_dark: Color,
    pub panel_bg: Color,
    pub sel_bg: Color,
    pub text_muted: Color,
    pub text_main: Color,
    pub blue: Color,
    pub green: Color,
    pub pink: Color,
    pub yellow: Color,
    pub red: Color,
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else if clean.len() == 3 {
        let r_char = &clean[0..1];
        let g_char = &clean[1..2];
        let b_char = &clean[2..3];
        let r = u8::from_str_radix(&format!("{}{}", r_char, r_char), 16).ok()?;
        let g = u8::from_str_radix(&format!("{}{}", g_char, g_char), 16).ok()?;
        let b = u8::from_str_radix(&format!("{}{}", b_char, b_char), 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}

impl Theme {
    pub fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Self::dark()
        } else {
            Self::light()
        }
    }

    pub fn resolve(option: crate::app::ThemeOption, custom_path: &std::path::Path) -> Self {
        match option {
            crate::app::ThemeOption::Dark => Self::dark(),
            crate::app::ThemeOption::Light => Self::light(),
            crate::app::ThemeOption::Custom => {
                if let Ok(theme) = Self::load_from_file(custom_path) {
                    theme
                } else {
                    // Fallback to dark
                    Self::dark()
                }
            }
        }
    }

    pub fn dark() -> Self {
        Self {
            bg_dark: Color::Rgb(17, 17, 27),       // Mocha deep background
            panel_bg: Color::Rgb(30, 30, 46),      // Mocha gray panel
            sel_bg: Color::Rgb(49, 50, 68),        // Mocha surface highlight
            text_muted: Color::Rgb(137, 143, 173), // Mocha muted text
            text_main: Color::White,
            blue: Color::Rgb(137, 180, 250),   // Mocha pastel blue
            green: Color::Rgb(166, 227, 161),  // Mocha pastel green
            pink: Color::Rgb(245, 194, 231),   // Mocha pastel pink
            yellow: Color::Rgb(249, 226, 175), // Mocha pastel yellow
            red: Color::Rgb(243, 139, 168),    // Mocha pastel red
        }
    }

    pub fn light() -> Self {
        Self {
            bg_dark: Color::Rgb(239, 241, 245),    // Latte off-white
            panel_bg: Color::Rgb(230, 233, 240),   // Latte light gray
            sel_bg: Color::Rgb(204, 208, 218),     // Latte surface highlight
            text_muted: Color::Rgb(108, 111, 133), // Latte darker muted gray
            text_main: Color::Rgb(76, 79, 105),    // Latte dark gray text
            blue: Color::Rgb(30, 102, 245),        // Latte rich blue
            green: Color::Rgb(64, 160, 43),        // Latte rich green
            pink: Color::Rgb(230, 72, 116),        // Latte rich pink
            yellow: Color::Rgb(223, 142, 29),      // Latte rich yellow
            red: Color::Rgb(210, 15, 57),          // Latte rich red
        }
    }

    pub fn load_from_file(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let parsed: serde_json::Value = serde_json::from_str(&content)?;

        let bg_dark = parsed
            .get("bg_dark")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(17, 17, 27));
        let panel_bg = parsed
            .get("panel_bg")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(30, 30, 46));
        let sel_bg = parsed
            .get("sel_bg")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(49, 50, 68));
        let text_muted = parsed
            .get("text_muted")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(137, 143, 173));
        let text_main = parsed
            .get("text_main")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::White);
        let blue = parsed
            .get("blue")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(137, 180, 250));
        let green = parsed
            .get("green")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(166, 227, 161));
        let pink = parsed
            .get("pink")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(245, 194, 231));
        let yellow = parsed
            .get("yellow")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(249, 226, 175));
        let red = parsed
            .get("red")
            .and_then(|v| v.as_str())
            .and_then(parse_hex_color)
            .unwrap_or(Color::Rgb(243, 139, 168));

        Ok(Self {
            bg_dark,
            panel_bg,
            sel_bg,
            text_muted,
            text_main,
            blue,
            green,
            pink,
            yellow,
            red,
        })
    }
}

/// Main UI render entry point
pub fn draw(f: &mut Frame, app: &mut App) {
    let theme = Theme::resolve(app.theme_option, &app.custom_theme_path);

    // Layout splits: Title, Main content viewport, Powerline Statusline
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title bar / header
            Constraint::Min(0),    // Main Workspace
            Constraint::Length(1), // Vim-style Powerline Statusline
        ])
        .split(f.area());

    // 1. Draw Sleek Header Title Bar (Dual-Tone Powerline wedge)
    let total_count = app.total_count;

    let title_spans = vec![
        Span::styled(
            " TRAZ ",
            Style::default()
                .bg(theme.blue)
                .fg(theme.bg_dark)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("", Style::default().bg(theme.panel_bg).fg(theme.blue)),
        Span::styled(
            "  Timeline Explorer  ",
            Style::default()
                .bg(theme.panel_bg)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("", Style::default().bg(theme.bg_dark).fg(theme.panel_bg)),
        Span::styled(
            format!("  {} events captured ", total_count),
            Style::default().fg(theme.text_muted),
        ),
    ];

    let title_bar =
        Paragraph::new(Line::from(title_spans)).style(Style::default().bg(theme.bg_dark));
    f.render_widget(title_bar, chunks[0]);

    // 2. Draw Main workspace area based on mode
    let main_area = chunks[1];
    let mode = app.mode.clone();
    match &mode {
        AppMode::List | AppMode::Search => {
            draw_list_view(f, app, &theme, main_area);
        }
        AppMode::Detail(id) => {
            draw_detail_view(f, app, *id, &theme, main_area);
        }
        AppMode::Diff(id) => {
            draw_diff_view(f, app, *id, &theme, main_area);
        }
        AppMode::Confirm(action) => {
            let prev_mode = app.previous_mode.clone();
            match prev_mode {
                Some(AppMode::Detail(id)) => {
                    draw_detail_view(f, app, id, &theme, main_area);
                }
                _ => {
                    draw_list_view(f, app, &theme, main_area);
                }
            }
            draw_confirm_popup(f, app, action, &theme, main_area);
        }
        AppMode::Settings => {
            let prev_mode = app.previous_mode.clone();
            match prev_mode {
                Some(AppMode::Detail(id)) => {
                    draw_detail_view(f, app, id, &theme, main_area);
                }
                _ => {
                    draw_list_view(f, app, &theme, main_area);
                }
            }
            draw_settings_popup(f, app, &theme, main_area);
        }
    }

    // 3. Draw Vim-like Powerline statusline at bottom
    let status_area = chunks[2];
    draw_status_bar(f, app, &theme, status_area);
}

/// Draw list view table with Catppuccin styles and beautiful prefix icons
fn draw_list_view(f: &mut Frame, app: &mut App, theme: &Theme, area: Rect) {
    let bg_block = Block::default().style(Style::default().bg(theme.bg_dark));
    f.render_widget(bg_block, area);

    if app.events.is_empty() {
        let empty_msg = if app.search_query.is_empty() {
            "  No events recorded in this database. Run `traz add` or capture commits to start.  "
        } else {
            "  No events found matching your active filter. Press [Esc] to reset.  "
        };
        let p = Paragraph::new(empty_msg)
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(theme.text_muted)
                    .add_modifier(Modifier::ITALIC),
            );
        f.render_widget(p, area);
        return;
    }

    let mut rows = Vec::new();
    for (i, event) in app.events.iter().enumerate() {
        let is_selected = i == app.selected;

        let mut row_cells = Vec::new();

        // Map icons and colors to event categories
        let (icon, type_color) = match event.event_type.as_str() {
            "git_commit" | "commit" => ("📝", theme.blue),
            "build_failure" => ("❌", theme.red),
            "branch_switch" => ("🌿", theme.yellow),
            "decision" => ("📌", theme.text_main),
            "test_run" | "test" => ("🧪", theme.green),
            _ => ("• ", theme.text_muted),
        };

        // 1. Selection and Index Gutter Column
        let select_sym = if is_selected { "▍" } else { " " };
        let select_color = if is_selected {
            theme.blue
        } else {
            theme.text_muted
        };

        let num_text = if app.show_gutters {
            format!("{} {:02} ", select_sym, i + 1)
        } else {
            format!("{} ", select_sym)
        };
        let num_span = Span::styled(
            num_text,
            Style::default()
                .fg(select_color)
                .add_modifier(Modifier::BOLD),
        );
        row_cells.push(Line::from(num_span));

        // 2. Timeline Tree Connection Column (Dynamic)
        if app.show_timeline {
            let (line_sym, bullet_sym) = if app.events.len() == 1 {
                (" ──", "◆ ")
            } else if i == 0 {
                (" ┌─", "◆ ")
            } else if i == app.events.len() - 1 {
                (" └─", "◆ ")
            } else {
                (" ├─", "◆ ")
            };
            row_cells.push(Line::from(vec![
                Span::styled(line_sym, Style::default().fg(theme.text_muted)),
                Span::styled(
                    bullet_sym,
                    Style::default().fg(type_color).add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            row_cells.push(Line::raw(""));
        }

        // 3. Event Title Column
        let title_content = format!("{} {}", icon, event.title);
        let title_span = Span::styled(
            title_content,
            Style::default().fg(type_color).add_modifier(Modifier::BOLD),
        );
        row_cells.push(Line::from(title_span));

        // 4. Tool Column
        let tool_span = Span::styled(
            format!(" [{}] ", event.tool),
            Style::default().fg(theme.text_muted),
        );
        row_cells.push(Line::from(tool_span));

        // 5. Time Column (or Score if searching)
        let relative = relative_time(&event.timestamp);
        let time_span = Span::styled(relative, Style::default().fg(theme.text_muted));
        row_cells.push(Line::from(time_span));

        // 6. Score Column (if available)
        if !app.search_scores.is_empty() {
            let score = app.search_scores.get(i).copied().unwrap_or(0.0);
            let score_pct = format!(" {:.0}% ", score * 100.0);
            let score_color = if score > 0.8 {
                theme.green
            } else if score > 0.5 {
                theme.yellow
            } else {
                theme.text_muted
            };
            row_cells.push(Line::from(Span::styled(
                score_pct,
                Style::default()
                    .fg(score_color)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        let row_style = if is_selected {
            Style::default().bg(theme.sel_bg).fg(Color::White)
        } else {
            Style::default().bg(theme.bg_dark).fg(Color::Gray)
        };

        rows.push(Row::new(row_cells).style(row_style));
    }

    // Dynamic width constraints based on settings
    let mut widths = if app.show_timeline {
        vec![
            Constraint::Length(if app.show_gutters { 6 } else { 2 }),
            Constraint::Length(5),
            Constraint::Percentage(if app.search_scores.is_empty() { 65 } else { 58 }),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
        ]
    } else {
        vec![
            Constraint::Length(if app.show_gutters { 6 } else { 2 }),
            Constraint::Length(0),
            Constraint::Percentage(if app.search_scores.is_empty() { 70 } else { 63 }),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
        ]
    };

    if !app.search_scores.is_empty() {
        widths.push(Constraint::Length(8));
    }

    let table = Table::new(rows, widths)
        .block(Block::default().borders(Borders::NONE))
        .row_highlight_style(Style::default())
        .highlight_symbol("");

    let mut state = TableState::default();
    state.select(Some(app.selected));

    f.render_stateful_widget(table, area, &mut state);
}

/// Draw the detailed view representing an interactive Vim buffer panel
fn draw_detail_view(f: &mut Frame, app: &mut App, id: i64, theme: &Theme, area: Rect) {
    let bg_block = Block::default().style(Style::default().bg(theme.bg_dark));
    f.render_widget(bg_block, area);

    let event_opt = app.all_events.iter().find(|e| e.id == Some(id));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.pink))
        .style(Style::default().bg(theme.panel_bg))
        .title(format!(" ⧉ event.detail • id: {} ", id));

    let event = match event_opt {
        Some(e) => e,
        None => {
            let p = Paragraph::new(format!(
                "Error: Event #{} is not present in local memory.",
                id
            ))
            .block(block)
            .style(Style::default().fg(theme.red));
            f.render_widget(p, area);
            return;
        }
    };

    let mut lines = Vec::new();
    lines.push(Line::raw(""));

    // 1. Premium Dual-Tone Header Block
    let badge_title = vec![
        Span::styled(
            " TITLE ",
            Style::default()
                .bg(theme.blue)
                .fg(theme.bg_dark)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}  ", event.title),
            Style::default()
                .bg(theme.sel_bg)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    lines.push(Line::from(badge_title));
    lines.push(Line::raw(""));

    // 2. Structured Metadata Badges
    let badge_tool = vec![
        Span::styled(
            " TOOL ",
            Style::default()
                .bg(theme.blue)
                .fg(theme.bg_dark)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", event.tool),
            Style::default().bg(theme.sel_bg).fg(Color::White),
        ),
    ];
    let badge_type = vec![
        Span::styled(
            " TYPE ",
            Style::default()
                .bg(theme.pink)
                .fg(theme.bg_dark)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", event.event_type),
            Style::default().bg(theme.sel_bg).fg(Color::White),
        ),
    ];
    let mut badge_line = vec![Span::raw("  ")];
    badge_line.extend(badge_tool);
    badge_line.push(Span::raw("    "));
    badge_line.extend(badge_type);
    lines.push(Line::from(badge_line));

    let formatted_time = event
        .created_at
        .unwrap_or(event.timestamp)
        .format("%b %d %Y %H:%M")
        .to_string();
    let badge_time = vec![
        Span::styled(
            " TIME ",
            Style::default()
                .bg(theme.text_muted)
                .fg(theme.bg_dark)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", formatted_time),
            Style::default().bg(theme.sel_bg).fg(Color::White),
        ),
    ];
    let tags_str = match &event.tags {
        Some(tags) if !tags.is_empty() => tags
            .iter()
            .map(|t| format!("#{}", t))
            .collect::<Vec<_>>()
            .join(" "),
        _ => "none".to_string(),
    };
    let badge_tags = vec![
        Span::styled(
            " TAGS ",
            Style::default()
                .bg(theme.yellow)
                .fg(theme.bg_dark)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", tags_str),
            Style::default().bg(theme.sel_bg).fg(Color::White),
        ),
    ];
    let mut badge_line2 = vec![Span::raw("  ")];
    badge_line2.extend(badge_time);
    badge_line2.push(Span::raw("    "));
    badge_line2.extend(badge_tags);
    lines.push(Line::from(badge_line2));
    lines.push(Line::raw(""));

    // 3. Changed Files Indent Tree
    lines.push(Line::from(Span::styled(
        "  📁 CHANGED FILES",
        Style::default()
            .fg(theme.text_muted)
            .add_modifier(Modifier::BOLD),
    )));
    if let Some(files) = &event.files {
        if files.is_empty() {
            lines.push(Line::from(Span::styled(
                "     (none)",
                Style::default()
                    .fg(theme.text_muted)
                    .add_modifier(Modifier::ITALIC),
            )));
        } else {
            for (idx, f) in files.iter().enumerate() {
                let guide = if idx == files.len() - 1 {
                    "     └── "
                } else {
                    "     ├── "
                };
                lines.push(Line::from(vec![
                    Span::styled(guide, Style::default().fg(theme.text_muted)),
                    Span::styled(f, Style::default().fg(theme.green)),
                ]));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "     (none)",
            Style::default()
                .fg(theme.text_muted)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    // Divider Line
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", "━".repeat(area.width.saturating_sub(6) as usize)),
        Style::default()
            .fg(theme.text_muted)
            .add_modifier(Modifier::DIM),
    )));
    lines.push(Line::raw(""));

    // 4. Blockquote Summary Pane
    lines.push(Line::from(Span::styled(
        "  📝 SUMMARY DESCRIPTION",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));
    if let Some(summary) = &event.summary {
        for l in summary.lines() {
            lines.push(Line::from(vec![
                Span::styled("     │  ", Style::default().fg(theme.blue)),
                Span::styled(l, Style::default().fg(theme.text_main)),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "     │  (no description provided for this event)",
            Style::default()
                .fg(theme.text_muted)
                .add_modifier(Modifier::ITALIC),
        )));
    }

    // 5. Compilation Session tree connections
    if event.event_type == "compilation_session" {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  ⚡ COMPILATION ATTRIBUTES",
            Style::default()
                .fg(theme.yellow)
                .add_modifier(Modifier::BOLD),
        )));

        if let Some(meta) = &event.metadata {
            let attempt = meta
                .get("attempt_count")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".to_string());
            let status = meta.get("status").and_then(|v| v.as_str()).unwrap_or("—");
            let started = meta
                .get("started_at")
                .and_then(|v| v.as_str())
                .unwrap_or("—");
            let resolved = meta
                .get("resolved_at")
                .and_then(|v| v.as_str())
                .unwrap_or("—");

            let status_color =
                if status.to_lowercase() == "success" || status.to_lowercase() == "resolved" {
                    theme.green
                } else if status.to_lowercase() == "failed" {
                    theme.red
                } else {
                    theme.yellow
                };

            lines.push(Line::from(vec![
                Span::styled("     ├─ Attempt:   ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    attempt,
                    Style::default()
                        .fg(theme.text_main)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("     ├─ Status:    ", Style::default().fg(theme.text_muted)),
                Span::styled(
                    status,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("     ├─ Started:   ", Style::default().fg(theme.text_muted)),
                Span::styled(started, Style::default().fg(theme.text_main)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("     └─ Resolved:  ", Style::default().fg(theme.text_muted)),
                Span::styled(resolved, Style::default().fg(theme.text_main)),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                "     (no session metadata details)",
                Style::default().fg(theme.text_muted),
            )));
        }
    }

    if event.diff.is_some() {
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  ℹ  Press [d] to open the code diff for this event.",
            Style::default()
                .fg(theme.text_muted)
                .add_modifier(Modifier::DIM),
        )));
    }

    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((app.scroll_offset as u16, 0));

    f.render_widget(p, area);
}

/// Draw the Diff view complete with a side-by-side vimdiff styled left gutter showing line numbers and gitgutter signals
fn draw_diff_view(f: &mut Frame, app: &mut App, id: i64, theme: &Theme, area: Rect) {
    let bg_block = Block::default().style(Style::default().bg(theme.bg_dark));
    f.render_widget(bg_block, area);

    let event_opt = app.all_events.iter().find(|e| e.id == Some(id));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.yellow))
        .style(Style::default().bg(theme.panel_bg))
        .title(format!(
            "  diffview ━ {} ",
            event_opt.map(|e| e.title.as_str()).unwrap_or("diff")
        ));

    let event = match event_opt {
        Some(e) => e,
        None => {
            let p = Paragraph::new(format!("Error: Event #{} has disappeared.", id))
                .block(block)
                .style(Style::default().fg(theme.red));
            f.render_widget(p, area);
            return;
        }
    };

    if let Some(diff_content) = &event.diff {
        let diff_lines = render_diff(diff_content);

        // Prepend custom vimdiff-like line number gutter with gitgutter symbols
        let mut styled_lines = Vec::new();
        for (idx, line) in diff_lines.into_iter().enumerate() {
            let line_color = line
                .spans
                .first()
                .and_then(|s| s.style.fg)
                .unwrap_or(Color::Reset);

            let (status_char, status_style) = if line_color == Color::Green {
                (
                    "+",
                    Style::default()
                        .fg(theme.green)
                        .add_modifier(Modifier::BOLD),
                )
            } else if line_color == Color::Red {
                (
                    "-",
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                )
            } else if line_color == Color::Cyan {
                (
                    "~",
                    Style::default().fg(theme.blue).add_modifier(Modifier::BOLD),
                )
            } else {
                (" ", Style::default())
            };

            let num_text = format!(" {:>3} │ ", idx + 1);
            let num_span = Span::styled(num_text, Style::default().fg(theme.text_muted));
            let status_span = Span::styled(status_char, status_style);
            let sep_span = Span::styled(" ", Style::default());

            // Extract the spans in the line
            let mut line_spans = vec![num_span, status_span, sep_span];
            for span in line.spans {
                line_spans.push(span);
            }
            styled_lines.push(Line::from(line_spans));
        }

        let p = Paragraph::new(styled_lines)
            .block(block)
            .scroll((app.scroll_offset as u16, 0));
        f.render_widget(p, area);
    } else {
        let p = Paragraph::new("No git patch or unified diff was recorded for this event.")
            .block(block)
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(theme.text_muted)
                    .add_modifier(Modifier::ITALIC),
            );
        f.render_widget(p, area);
    }
}

/// Draw the centered confirm popup dialog box
fn draw_confirm_popup(f: &mut Frame, app: &App, action: &ConfirmAction, theme: &Theme, area: Rect) {
    let popup_rect = centered_rect(45, 8, area);
    f.render_widget(Clear, popup_rect);

    let popup_block = Block::default()
        .title(" ⚠️  Confirm Action ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.panel_bg));

    let mut lines = Vec::new();
    lines.push(Line::raw(""));

    match action {
        ConfirmAction::Undo(id) => {
            let title = app
                .all_events
                .iter()
                .find(|e| e.id == Some(*id))
                .map(|e| e.title.as_str())
                .unwrap_or("Unknown event");
            lines.push(Line::from(
                format!("   Delete event #{}?", id).white().bold(),
            ));
            lines.push(Line::from(
                format!("   \"{}\"", title).fg(theme.yellow).italic(),
            ));
            lines.push(Line::from(
                "   This deletion is irreversible.".fg(theme.red),
            ));
        }
        ConfirmAction::Rewind(id) => {
            let count = app.rewind_count;
            lines.push(Line::from(
                format!("   Rewind history to event #{}?", id)
                    .white()
                    .bold(),
            ));
            lines.push(Line::from(
                format!("   This will delete {} future events.", count)
                    .fg(theme.red)
                    .bold(),
            ));
            lines.push(Line::from(
                "   This modification is irreversible.".fg(theme.red),
            ));
        }
        ConfirmAction::Compress => {
            lines.push(Line::from(
                "   Compress events older than 14 days?".white().bold(),
            ));
            lines.push(Line::from(
                "   Creates an epoch checkpoint summary.".fg(theme.blue),
            ));
            lines.push(Line::from(
                "   Older entries will be collapsed.".fg(theme.red),
            ));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(
            "     [y] ",
            Style::default()
                .fg(theme.green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("confirm         "),
        Span::styled(
            "[n / Esc] ",
            Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
        ),
        Span::raw("cancel"),
    ]));

    let p = Paragraph::new(lines).block(popup_block);
    f.render_widget(p, popup_rect);
}

/// Draw the centered Settings modal panel
fn draw_settings_popup(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let popup_rect = centered_rect(48, 11, area);
    f.render_widget(Clear, popup_rect);

    let popup_block = Block::default()
        .title(" ⚙️  Traz Preferences ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.pink).add_modifier(Modifier::BOLD))
        .style(Style::default().bg(theme.panel_bg));

    let mut lines = Vec::new();
    lines.push(Line::raw(""));

    let theme_val = match app.theme_option {
        crate::app::ThemeOption::Dark => "🌙 Catppuccin Dark",
        crate::app::ThemeOption::Light => "☀️ Catppuccin Light",
        crate::app::ThemeOption::Custom => "🎨 Custom (theme.json)",
    };

    let settings_items = [
        ("Theme Palette", theme_val),
        (
            "Timeline Connection Trees",
            if app.show_timeline {
                "✓ Connected"
            } else {
                "✗ Disabled"
            },
        ),
        (
            "Line Numbering Gutters",
            if app.show_gutters {
                "✓ Active"
            } else {
                "✗ Hidden"
            },
        ),
    ];

    for (idx, (label, val)) in settings_items.iter().enumerate() {
        let is_selected = idx == app.selected_setting;

        let indicator = if is_selected { " ❯ " } else { "   " };
        let item_color = if is_selected {
            theme.pink
        } else {
            theme.text_muted
        };

        let label_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_main)
        };

        let val_color = if is_selected { theme.green } else { theme.blue };
        let val_style = Style::default().fg(val_color).add_modifier(Modifier::BOLD);

        let row_style = if is_selected {
            Style::default().bg(theme.sel_bg)
        } else {
            Style::default()
        };

        lines.push(
            Line::from(vec![
                Span::styled(
                    indicator,
                    Style::default().fg(item_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {:<30}", label), label_style),
                Span::styled(format!(" [ {} ] ", val), val_style),
            ])
            .style(row_style),
        );
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "    j/k Navigate  •  Space/Enter Toggle  •  Esc Save & Close",
        Style::default()
            .fg(theme.text_muted)
            .add_modifier(Modifier::DIM),
    )));

    let p = Paragraph::new(lines).block(popup_block);
    f.render_widget(p, popup_rect);
}

/// Helper function to center a popup rectangle relative to parent layout
fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Draw a highly-customized, beautiful Vim-like Powerline lualine split status bar
fn draw_status_bar(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(35)])
        .split(area);

    // Left status panel (Workspace context, modes, messages)
    let mut left_spans = Vec::new();

    let (mode_lbl, mode_color) = match app.mode {
        AppMode::List => (" NORMAL ", theme.blue),
        AppMode::Search => (" SEARCH ", theme.green),
        AppMode::Detail(_) => (" BUFFER ", theme.pink),
        AppMode::Diff(_) => ("  DIFF  ", theme.yellow),
        AppMode::Confirm(_) => (" WARNING", theme.red),
        AppMode::Settings => ("SETTINGS", theme.pink),
    };

    left_spans.push(Span::styled(
        mode_lbl,
        Style::default()
            .bg(mode_color)
            .fg(theme.bg_dark)
            .add_modifier(Modifier::BOLD),
    ));

    // Powerline solid wedge from mode label to panel segment
    left_spans.push(Span::styled(
        "",
        Style::default().bg(theme.panel_bg).fg(mode_color),
    ));

    // Database segment
    left_spans.push(Span::styled(
        " 📁 traz.db ",
        Style::default()
            .bg(theme.panel_bg)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));
    left_spans.push(Span::styled(
        "",
        Style::default().bg(theme.bg_dark).fg(theme.panel_bg),
    ));

    // Search query or context message
    if app.mode == AppMode::Search {
        left_spans.push(Span::styled(
            "  /",
            Style::default()
                .fg(theme.green)
                .add_modifier(Modifier::BOLD),
        ));
        left_spans.push(Span::styled(
            &app.search_query,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));
        left_spans.push(Span::styled("█", Style::default().fg(theme.green)));
    } else if let Some(status_msg) = &app.status_message {
        left_spans.push(Span::styled(
            format!("  {}", status_msg),
            Style::default()
                .fg(theme.green)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        let selection_lbl = if !app.events.is_empty() {
            format!("  [{:02}/{:02}]  ", app.selected + 1, app.events.len())
        } else {
            "  [00/00]  ".to_string()
        };
        left_spans.push(Span::styled(
            selection_lbl,
            Style::default().fg(theme.text_muted),
        ));

        // Mode hints on the left in small text
        let hints = match app.mode {
            AppMode::List => {
                "j/k Down/Up  •  Enter Detail  •  d Diff  •  s Settings  •  u Undo  •  r Rewind"
            }
            AppMode::Detail(_) => "j/k Scroll  •  Esc List  •  d Diff  •  u Undo  •  r Rewind",
            AppMode::Diff(_) => "j/k Scroll  •  Esc Detail",
            AppMode::Confirm(_) => "y Confirm  •  n/Esc Cancel",
            AppMode::Search => unreachable!(),
            AppMode::Settings => "j/k Navigate  •  Space Toggle  •  Esc Save & Close",
        };
        left_spans.push(Span::styled(hints, Style::default().fg(theme.text_muted)));
    }

    let left_bar = Paragraph::new(Line::from(left_spans)).style(Style::default().bg(theme.bg_dark));
    f.render_widget(left_bar, status_layout[0]);

    // Right status panel (System context & theme tag)
    let mut right_spans = Vec::new();
    right_spans.push(Span::styled(
        "",
        Style::default().bg(theme.bg_dark).fg(theme.panel_bg),
    ));

    let theme_lbl = match app.theme_option {
        crate::app::ThemeOption::Dark => " 🌙 Dark ",
        crate::app::ThemeOption::Light => " ☀️ Light ",
        crate::app::ThemeOption::Custom => " 🎨 Custom ",
    };
    right_spans.push(Span::styled(
        theme_lbl,
        Style::default().bg(theme.panel_bg).fg(Color::White),
    ));

    right_spans.push(Span::styled(
        "",
        Style::default().bg(theme.panel_bg).fg(mode_color),
    ));
    right_spans.push(Span::styled(
        "  ⧉ traz  ",
        Style::default()
            .bg(mode_color)
            .fg(theme.bg_dark)
            .add_modifier(Modifier::BOLD),
    ));

    let right_bar = Paragraph::new(Line::from(right_spans))
        .alignment(Alignment::Right)
        .style(Style::default().bg(theme.bg_dark));
    f.render_widget(right_bar, status_layout[1]);
}

/// Generate a human-friendly relative time string
fn relative_time(then: &chrono::DateTime<chrono::Utc>) -> String {
    let delta = chrono::Utc::now().signed_duration_since(*then);

    if delta.num_seconds() < 60 {
        "just now".to_string()
    } else if delta.num_minutes() < 60 {
        format!("{}m ago", delta.num_minutes())
    } else if delta.num_hours() < 24 {
        format!("{}h ago", delta.num_hours())
    } else {
        format!("{}d ago", delta.num_days())
    }
}
