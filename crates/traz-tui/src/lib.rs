use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;

pub mod app;
pub mod cuby_tui;
pub mod diff;
pub mod input;
pub mod ui;

use app::App;
use input::handle_input;
use ui::draw;

/// RAII Guard that ensures raw mode is disabled and alternate screen is left on drop/panic.
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().map_err(|_| {
            eprintln!("Error: traz tui requires raw mode terminal support.");
            eprintln!("Try running in a local terminal, not over pipe or dumb terminal.");
            std::process::exit(1);
        })?;

        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        Ok(RawModeGuard)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}

/// Run the TUI timeline explorer dashboard.
pub fn run(db_path: PathBuf) -> Result<()> {
    // 1. Startup Sequence: Print experimental guide to plain stdout
    println!();
    println!(
        "  \x1b[38;5;51m◆\x1b[0m \x1b[1mtraz-tui\x1b[0m \x1b[38;5;240m•\x1b[0m interactive timeline explorer \x1b[38;5;245m(v{})\x1b[0m",
        env!("CARGO_PKG_VERSION")
    );
    println!("  \x1b[38;5;240m├─\x1b[0m \x1b[38;5;77m✓\x1b[0m local-first engine online");
    println!(
        "  \x1b[38;5;240m├─\x1b[0m \x1b[38;5;245mnavigate:  \x1b[1mj\x1b[0m/\x1b[1mk\x1b[0m or \x1b[1m↑\x1b[0m/\x1b[1m↓\x1b[0m  │  select: \x1b[1mEnter\x1b[0m  │  settings: \x1b[1ms\x1b[0m or \x1b[1m,\x1b[0m"
    );
    println!(
        "  \x1b[38;5;240m└─\x1b[0m \x1b[38;5;245mcommands:  \x1b[1md\x1b[0m diff      │  \x1b[1mu\x1b[0m undo     │  \x1b[1mr\x1b[0m rewind    │  \x1b[1mq\x1b[0m quit"
    );
    println!();
    println!("  \x1b[38;5;208m⚡ Entering visual workspace...\x1b[0m");
    let _ = std::io::stdout().flush();

    // Sleep 1800ms
    std::thread::sleep(std::time::Duration::from_millis(1800));

    // Clear lines programmatically
    for _ in 0..8 {
        print!("\x1b[1A\x1b[2K");
    }
    let _ = std::io::stdout().flush();

    // 2. Open DB directly
    let db = traz_db::Db::open(&db_path)?;

    let custom_theme_path = db_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("theme.json");

    // 3. Load last 100 events
    let events = db.get_recent_events(100)?;
    let mut app = App::new(db, events, custom_theme_path);

    // 4. Enter raw mode using the RAII guard
    let _guard = RawModeGuard::new()?;

    // Setup Ratatui terminal
    let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    // 5. Enter main event loop
    loop {
        // Periodically clear transient status message
        app.check_status_message();

        terminal.draw(|f| {
            draw(f, &mut app);
        })?;

        if crossterm::event::poll(std::time::Duration::from_millis(50))?
            && let crossterm::event::Event::Key(key) = crossterm::event::read()?
            && key.kind != crossterm::event::KeyEventKind::Release
            && handle_input(&mut app, key)?
        {
            break;
        }
    }

    Ok(())
}
