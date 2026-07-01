use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use std::path::PathBuf;
use std::sync::Arc;
use traz_db::Db;

/// State for the Cuby TUI Pet Game.
pub struct CubyAppState {
    pub db: Arc<Db>,
    pub happiness: u32,
    pub hunger: u32,
    pub brain_power: u32,
    pub mood: String,
    pub dialogue: String,
    pub current_action: String,
    pub dance_ticks: u32,
    pub dance_frame: usize,
    pub pet_ticks: u32,
    pub feed_ticks: u32,
    pub poke_ticks: u32,
    pub quit: bool,
    pub ask_mode: bool,
    pub query_buffer: String,
    pub search_results: Vec<String>,
}

impl CubyAppState {
    pub async fn new(db: Arc<Db>) -> Self {
        let count = db.count_events().await.unwrap_or(0);
        let natural_mood = if count == 0 {
            "sad"
        } else if count <= 10 {
            "happy"
        } else if count <= 50 {
            "excited"
        } else {
            "chill"
        };

        let initial_dialogue = if count == 0 {
            "Welcome! My memory is empty. Feed me some context treats! 🥺"
        } else {
            "I'm ready! Let's write some beautiful software!"
        };

        Self {
            db,
            happiness: 80,
            hunger: 20,
            brain_power: 50,
            mood: natural_mood.to_string(),
            dialogue: initial_dialogue.to_string(),
            current_action: String::new(),
            dance_ticks: 0,
            dance_frame: 0,
            pet_ticks: 0,
            feed_ticks: 0,
            poke_ticks: 0,
            quit: false,
            ask_mode: false,
            query_buffer: String::new(),
            search_results: Vec::new(),
        }
    }

    pub fn get_natural_mood(&self) -> String {
        if self.hunger > 60 {
            "sad".to_string()
        } else if self.happiness > 80 {
            "happy".to_string()
        } else if self.brain_power > 80 {
            "excited".to_string()
        } else {
            "neutral".to_string()
        }
    }
}

/// RAII Guard that ensures raw mode is disabled and alternate screen is left on drop/panic.
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}

/// Run the Ratatui-based Cuby Pet game.
pub async fn run_cuby_game(db_path: PathBuf) -> Result<()> {
    let db = Arc::new(Db::open(&db_path).await?);
    let mut app_state = CubyAppState::new(db).await;

    let _guard = RawModeGuard::new()?;
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let poll_dur = std::time::Duration::from_millis(150);
    let mut loop_count = 0;
    let mut blink_ticks = 0;

    loop {
        loop_count += 1;

        // ── Async Game Tick Animations ──
        if app_state.dance_ticks > 0 {
            app_state.dance_ticks -= 1;
            app_state.dance_frame += 1;

            let frames = ["wink", "wink_right", "happy", "excited"];
            app_state.mood = frames[app_state.dance_frame % frames.len()].to_string();
            app_state.dialogue = "🎶 Dancing, dancing! Feel the context beat! 🎶".to_string();
        } else if app_state.current_action == "dance" {
            app_state.current_action = String::new();
            app_state.mood = app_state.get_natural_mood();
            app_state.dialogue = "Whew! That was a spectacular dance! 💃".to_string();
        } else if app_state.pet_ticks > 0 {
            app_state.pet_ticks -= 1;
            let moods = ["wink", "wink_right", "happy", "wink", "happy", "default"];
            app_state.mood = moods[app_state.pet_ticks as usize % moods.len()].to_string();
            app_state.dialogue = "Purrrr... *Happy winks* 🥰".to_string();
        } else if app_state.feed_ticks > 0 {
            app_state.feed_ticks -= 1;
            let moods = ["asleep", "derp", "asleep", "surprised", "happy", "default"];
            app_state.mood = moods[app_state.feed_ticks as usize % moods.len()].to_string();
            app_state.dialogue = "*NOM NOM NOM* chewing yummy context... 🍕".to_string();
        } else if app_state.poke_ticks > 0 {
            app_state.poke_ticks -= 1;
            let moods = ["surprised", "dizzy", "angry", "derp"];
            app_state.mood = moods[app_state.poke_ticks as usize % moods.len()].to_string();
            app_state.dialogue = "Hey! That tickles! ⚡".to_string();
        } else if app_state.current_action == "pet" {
            let dialog = get_context_aware_dialog(&app_state, "pet").await;
            app_state.current_action = String::new();
            app_state.mood = app_state.get_natural_mood();
            app_state.dialogue = dialog;
        } else if app_state.current_action == "feed" {
            let dialog = get_context_aware_dialog(&app_state, "feed").await;
            app_state.current_action = String::new();
            app_state.mood = app_state.get_natural_mood();
            app_state.dialogue = dialog;
        } else if app_state.current_action == "poke" {
            let dialog = get_context_aware_dialog(&app_state, "poke").await;
            app_state.current_action = String::new();
            app_state.mood = app_state.get_natural_mood();
            app_state.dialogue = dialog;
        } else if app_state.current_action == "clean" {
            let dialog = get_context_aware_dialog(&app_state, "clean").await;
            app_state.current_action = String::new();
            app_state.mood = app_state.get_natural_mood();
            app_state.dialogue = dialog;
        } else {
            // ── Idle Blinking Logic ──
            if loop_count % 25 == 0 {
                blink_ticks = 2;
            }

            if blink_ticks > 0 {
                blink_ticks -= 1;
                app_state.mood = "asleep".to_string();
            } else {
                app_state.mood = app_state.get_natural_mood();
            }
        }

        terminal.draw(|f| {
            render_cuby_ui(f, &mut app_state);
        })?;

        if event::poll(poll_dur)?
            && let Event::Key(key) = event::read()?
            && key.kind != KeyEventKind::Release
        {
            handle_cuby_key(&mut app_state, key.code).await?;
        }

        if app_state.quit {
            break;
        }
    }

    Ok(())
}

async fn handle_cuby_key(app: &mut CubyAppState, code: KeyCode) -> Result<()> {
    if app.ask_mode {
        match code {
            KeyCode::Esc => {
                app.ask_mode = false;
                app.dialogue = "Back to chilling! What else shall we do?".to_string();
                app.mood = app.get_natural_mood();
            }
            KeyCode::Enter if !app.query_buffer.is_empty() => {
                let query = app.query_buffer.clone();
                app.query_buffer.clear();

                let filters = traz_db::SearchFilters::default();
                let results = app.db.search_events(&query, &filters, 5).await?;

                app.search_results.clear();
                if results.is_empty() {
                    app.mood = "dizzy".to_string();
                    app.dialogue =
                        format!("Oh no, I couldn't find any memories matching '{}'!", query);
                } else {
                    app.mood = "suspicious".to_string();
                    app.dialogue = format!("I sniffed out {} matches for you!", results.len());
                    for (idx, event) in results.iter().enumerate() {
                        let rel = relative_time_string(&event.timestamp);
                        let icon = type_icon_string(&event.event_type);
                        app.search_results.push(format!(
                            "    {}. {} {} ({}) [{}]",
                            idx + 1,
                            icon,
                            event.title,
                            rel,
                            event.tool
                        ));
                    }
                }
            }
            KeyCode::Char(c) => {
                app.query_buffer.push(c);
            }
            KeyCode::Backspace => {
                app.query_buffer.pop();
            }
            _ => {}
        }
        return Ok(());
    }

    match code {
        KeyCode::Char('1') | KeyCode::Char('p') | KeyCode::Char('P') => {
            app.happiness = (app.happiness + 15).min(100);
            app.hunger = (app.hunger + 5).min(100);
            app.pet_ticks = 6;
            app.current_action = "pet".to_string();
        }
        KeyCode::Char('2') | KeyCode::Char('f') | KeyCode::Char('F') => {
            app.hunger = (app.hunger as i32 - 25).max(0) as u32;
            app.brain_power = (app.brain_power + 10).min(100);
            app.feed_ticks = 6;
            app.current_action = "feed".to_string();
        }
        KeyCode::Char('3') | KeyCode::Char('s') | KeyCode::Char('S') => {
            app.happiness = (app.happiness + 10).min(100);
            app.mood = "happy".to_string();
            app.current_action = "sing".to_string();
            app.dialogue = format!("🎶 {}", get_random_song());
        }
        KeyCode::Char('4') | KeyCode::Char('d') | KeyCode::Char('D') => {
            app.happiness = (app.happiness + 20).min(100);
            app.hunger = (app.hunger + 10).min(100);
            app.current_action = "dance".to_string();
            app.dance_ticks = 16;
            app.dance_frame = 0;
        }
        KeyCode::Char('5') | KeyCode::Char('k') | KeyCode::Char('K') => {
            app.happiness = (app.happiness + 10).min(100);
            app.poke_ticks = 4;
            app.current_action = "poke".to_string();
        }
        KeyCode::Char('6') | KeyCode::Char('c') | KeyCode::Char('C') => {
            app.brain_power = (app.brain_power + 15).min(100);
            app.mood = "focused".to_string();
            app.current_action = "clean".to_string();
            app.dialogue =
                "Vacuuming the context vault... Clearing old cached files! 🧹".to_string();
        }
        KeyCode::Char('7') | KeyCode::Char('a') | KeyCode::Char('A') => {
            app.ask_mode = true;
            app.query_buffer.clear();
            app.search_results.clear();
            app.mood = "curious".to_string();
            app.dialogue = "Sniffing database... What keyword should I search for?".to_string();
        }
        KeyCode::Char('8') | KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
            app.quit = true;
        }
        _ => {}
    }

    Ok(())
}

fn get_cuby_ascii_spans<'a>(expression: &str, cuby_color: Color) -> Vec<Line<'a>> {
    let screen_color = Color::DarkGray;
    let eye_color = Color::White;

    let mut lines = Vec::new();

    // Spacing blank line to align Cuby nicely with the borderless stats
    lines.push(Line::from(""));

    // Line 0: "  ▄▄▄▄▄▄▄▄▄▄"
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("▄▄▄▄▄▄▄▄▄▄", Style::default().fg(cuby_color)),
    ]));

    // Line 1: " █ ┌──────┐ █"
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled("█", Style::default().fg(cuby_color)),
        Span::raw(" "),
        Span::styled("┌──────┐", Style::default().fg(screen_color)),
        Span::raw(" "),
        Span::styled("█", Style::default().fg(cuby_color)),
    ]));

    // Line 2: " █ │ <eyes> │ █"
    let eyes = match expression {
        "neutral" | "default" => "■  ■",
        "happy" | "curious" => "^  ^",
        "sad" | "sleepy" => "▄  ▄",
        "angry" | "scowling" => ">  <",
        "surprised" | "shocked" => "O  O",
        "wink" | "mischievous" => "■  -",
        "wink_right" => "-  ■",
        "suspicious" | "processing" => "▀  ■",
        "error" | "offline" => "X  X",
        "excited" | "starstruck" => "*  *",
        "dizzy" | "confused" => "@  @",
        "asleep" | "idle" => "-  -",
        "crying" | "overwhelmed" => "T  T",
        "focused" | "determined" => "=  =",
        "derp" | "buggy" => "O  o",
        "chill" | "content" => "~  ~",
        "premium" | "monetized" => "$  $",
        _ => "■  ■",
    };

    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled("█", Style::default().fg(cuby_color)),
        Span::raw(" "),
        Span::styled("│", Style::default().fg(screen_color)),
        Span::raw(" "),
        Span::styled(
            eyes,
            Style::default().fg(eye_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("│", Style::default().fg(screen_color)),
        Span::raw(" "),
        Span::styled("█", Style::default().fg(cuby_color)),
    ]));

    // Line 3: " █ └──────┘ █"
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled("█", Style::default().fg(cuby_color)),
        Span::raw(" "),
        Span::styled("└──────┘", Style::default().fg(screen_color)),
        Span::raw(" "),
        Span::styled("█", Style::default().fg(cuby_color)),
    ]));

    // Line 4: "  ▀▀▀▀▀▀▀▀▀▀"
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("▀▀▀▀▀▀▀▀▀▀", Style::default().fg(cuby_color)),
    ]));

    // Line 5: "    ▀    ▀"
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled("▀", Style::default().fg(cuby_color)),
        Span::raw("    "),
        Span::styled("▀", Style::default().fg(cuby_color)),
    ]));

    lines
}

fn render_cuby_ui(f: &mut ratatui::Frame, app: &mut CubyAppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(16), Constraint::Min(20)])
        .split(f.area());

    let cuby_color = match app.mood.as_str() {
        "happy" | "curious" | "excited" | "starstruck" => Color::Cyan,
        "sad" | "sleepy" | "crying" | "overwhelmed" | "asleep" => Color::Blue,
        "angry" | "scowling" | "error" | "offline" => Color::Magenta,
        "dizzy" | "confused" | "premium" | "monetized" => Color::Yellow,
        "chill" | "content" => Color::Cyan,
        _ => Color::Blue,
    };

    let cuby_spans = get_cuby_ascii_spans(&app.mood, cuby_color);
    let cuby_paragraph = Paragraph::new(cuby_spans);
    f.render_widget(cuby_paragraph, chunks[0]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Min(4),
        ])
        .split(chunks[1]);

    // ── Stats Section (Borderless!) ──
    let mut stats_lines = vec![Line::from(vec![
        Span::styled(
            "── Play with Cuby! ── ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Blinking Idle Mode Enabled ✨",
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    let happy_bar = draw_bar_string(app.happiness);
    let hunger_bar = draw_bar_string(app.hunger);
    let brain_bar = draw_bar_string(app.brain_power);

    stats_lines.push(Line::from(vec![
        Span::styled("Happiness: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} {}%", happy_bar, app.happiness),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    stats_lines.push(Line::from(vec![
        Span::styled("Hunger:    ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} {}%", hunger_bar, app.hunger),
            Style::default().fg(Color::Blue),
        ),
    ]));
    stats_lines.push(Line::from(vec![
        Span::styled("Brain IQ:  ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} {}%", brain_bar, app.brain_power),
            Style::default().fg(Color::Magenta),
        ),
    ]));

    let stats_paragraph = Paragraph::new(stats_lines);
    f.render_widget(stats_paragraph, right_chunks[0]);

    // ── Thoughts / Dialogue Section (Borderless, formatted in neat quotes) ──
    let dialog_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "“",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.dialogue),
            Span::styled(
                "”",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let dialog_paragraph = Paragraph::new(dialog_text).wrap(Wrap { trim: true });
    f.render_widget(dialog_paragraph, right_chunks[1]);

    // ── Action Menu / Input Section (Borderless) ──
    if app.ask_mode {
        let ask_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(3)])
            .split(right_chunks[2]);

        let input_text = vec![
            Line::from(vec![
                Span::styled(
                    "  Query: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&app.query_buffer),
                Span::styled("▋", Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![Span::styled(
                "  (Type keyword and press Enter. Esc to go back)",
                Style::default().fg(Color::DarkGray),
            )]),
        ];
        let input_p = Paragraph::new(input_text);
        f.render_widget(input_p, ask_layout[0]);

        let mut results_lines = vec![Line::from("  ── Search Results ──")];
        if app.search_results.is_empty() {
            results_lines.push(Line::from(
                "    Type a keyword above to search database context...",
            ));
        } else {
            for res in &app.search_results {
                results_lines.push(Line::from(res.as_str()));
            }
        }
        let results_p = Paragraph::new(results_lines);
        f.render_widget(results_p, ask_layout[1]);
    } else {
        let menu_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "  What should we do?",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  1. ", Style::default().fg(Color::Green)),
                Span::raw("Pet Cuby 👋         "),
                Span::styled("  2. ", Style::default().fg(Color::Green)),
                Span::raw("Feed Context Treat 🍕"),
            ]),
            Line::from(vec![
                Span::styled("  3. ", Style::default().fg(Color::Green)),
                Span::raw("Sing a Song 🎶       "),
                Span::styled("  4. ", Style::default().fg(Color::Green)),
                Span::raw("Let's Dance! 💃"),
            ]),
            Line::from(vec![
                Span::styled("  5. ", Style::default().fg(Color::Green)),
                Span::raw("Poke Cuby 👉        "),
                Span::styled("  6. ", Style::default().fg(Color::Green)),
                Span::raw("Clean Vault 🧹"),
            ]),
            Line::from(vec![
                Span::styled("  7. ", Style::default().fg(Color::Green)),
                Span::raw("Ask a Question 🔍    "),
                Span::styled("  8. ", Style::default().fg(Color::Red)),
                Span::raw("Exit Game 🚪"),
            ]),
        ];
        let menu_p = Paragraph::new(menu_text);
        f.render_widget(menu_p, right_chunks[2]);
    }
}

fn draw_bar_string(percentage: u32) -> String {
    let filled = (percentage / 10) as usize;
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn relative_time_string(then: &chrono::DateTime<chrono::Utc>) -> String {
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

fn type_icon_string(event_type: &str) -> &'static str {
    match event_type {
        "bug_fix" => "🐛",
        "feature" => "✨",
        "refactor" => "♻️ ",
        "decision" => "📌",
        "commit" => "📝",
        "debug" => "🔍",
        "test" => "🧪",
        "deploy" => "🚀",
        "revert" => "⏪",
        _ => "•",
    }
}

fn get_random_song() -> &'static str {
    let songs = [
        "Code compiles at midnight clear, / No single warning to cause us fear! 🎶",
        "Soft keyboard clicks, a quiet screen, / Sleekest binary you've ever seen! 💻",
        "Git merge conflicts melt away, / It's a gorgeous green deployment day! 🚀",
        "Oh database, oh SQLite store, / Save my traces forevermore! 📁",
        "Haiku: Bug in the system, / Squashed by a developer, / Cuby remembers. 🌸",
    ];
    let idx = (chrono::Utc::now().timestamp_subsec_millis() as usize) % songs.len();
    songs[idx]
}

async fn get_context_aware_dialog(app: &CubyAppState, action: &str) -> String {
    let recent = app.db.get_recent_events(3).await.unwrap_or_default();
    let last_title = recent
        .first()
        .map(|e| e.title.as_str())
        .unwrap_or("writing cool code");
    let last_tool = recent.first().map(|e| e.tool.as_str()).unwrap_or("traz");

    let idx = (chrono::Utc::now().timestamp_subsec_millis() as usize) % 5;

    match action {
        "pet" => {
            let responses = [
                "Purrrr... Your cursor is so warm! Happiness increased! 🥰".to_string(),
                format!(
                    "Ah, that's the spot! Love watching you use {} for '{}'! 👋",
                    last_tool, last_title
                ),
                "Beep boop! Cuby rolls over. That felt as good as your last bug fix! ✨"
                    .to_string(),
                format!(
                    "*Happy wiggles* I promise to remember '{}' forever! ⚡",
                    last_title
                ),
                format!(
                    "*nuzzles* Cozy terminal! Let's write more code using {}!",
                    last_tool
                ),
            ];
            responses[idx].clone()
        }
        "feed" => {
            let responses = [
                "*Nom nom nom* Swallowed a trace treat! Hunger decreased! 🍕".to_string(),
                format!(
                    "Yum! Tastes almost as good as the '{}' event you logged! 💾",
                    last_title
                ),
                format!(
                    "*Chomps* Ah, packed with {} metadata! Hunger -25%! 🧠",
                    last_tool
                ),
                format!(
                    "Delicious treat! Ready to remember all your '{}' traces! ⚡",
                    last_title
                ),
                "*Nom* That trace had a delicious flavor profile! My brain is growing! 🌟"
                    .to_string(),
            ];
            responses[idx].clone()
        }
        "poke" => {
            let responses = [
                "Hey! That tickles! ⚡".to_string(),
                "Ouch! Be gentle with my context shell! 🥺".to_string(),
                format!(
                    "Stop poking! I'm trying to think about '{}'! 🧠",
                    last_title
                ),
                "Beep boop! Cuby giggles and wiggles away! 😂".to_string(),
                format!("Ah! A poke! Is that a bug in '{}' I smell?", last_title),
            ];
            responses[idx].clone()
        }
        "clean" => {
            let responses = [
                "Vacuuming the context vault... Clearing old cached files! 🧹".to_string(),
                "Sweeping away unused traces... SQLite is shiny and clean! ✨".to_string(),
                "Garbage collection complete! Optimized all traces! ⚙️".to_string(),
                "Dusting off the indexes... Cuby feels so refreshed! 🧘".to_string(),
                format!("Optimized {} database blocks. Brain power +15%!", last_tool),
            ];
            responses[idx].clone()
        }
        _ => "I'm ready! Let's write some beautiful software!".to_string(),
    }
}
