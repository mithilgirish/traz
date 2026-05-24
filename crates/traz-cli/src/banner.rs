use std::io::IsTerminal;

/// The traz ASCII art banner with gradient coloring
pub fn print_banner() {
    let is_color = std::io::stdout().is_terminal();

    if is_color {
        print_color_banner();
    } else {
        print_plain_banner();
    }
}

fn print_color_banner() {
    // ── Sleek block-letter TRAZ with cyan-to-magenta gradient ──
    let art: &[(&str, &str)] = &[
        (
            "\x1b[38;5;51m",
            r#"          ████████╗██████╗   █████╗  ███████╗"#,
        ),
        (
            "\x1b[38;5;45m",
            r#"          ╚══██╔══╝██╔══██╗ ██╔══██╗ ╚══███╔╝"#,
        ),
        (
            "\x1b[38;5;39m",
            r#"             ██║   ██████╔╝ ███████║   ███╔╝ "#,
        ),
        (
            "\x1b[38;5;33m",
            r#"             ██║   ██╔══██╗ ██╔══██║  ███╔╝  "#,
        ),
        (
            "\x1b[38;5;63m",
            r#"             ██║   ██║  ██║ ██║  ██║ ███████╗"#,
        ),
        (
            "\x1b[38;5;99m",
            r#"             ╚═╝   ╚═╝  ╚═╝ ╚═╝  ╚═╝ ╚══════╝"#,
        ),
    ];

    println!();
    for (color, line) in art {
        println!("{}{}\x1b[0m", color, line);
    }

    println!("\n  \x1b[38;5;240m─────────────────────────────────────────────────────\x1b[0m");

    println!();
}

fn print_plain_banner() {
    println!();
    println!(r#"  ╔══════════════════════════════════════════════╗"#);
    println!(r#"  ║   TRAZ — developer memory for AI tools      ║"#);
    println!(r#"  ╚══════════════════════════════════════════════╝"#);
    println!();
    let version = env!("CARGO_PKG_VERSION");
    println!("  v{}  |  local-first  |  zero-cloud  |  MIT", version);
    println!();
}

pub fn print_interactive_welcome() {
    let is_color = std::io::stdout().is_terminal();

    if is_color {
        println!(
            "  \x1b[38;5;245mInteractive mode. Type \x1b[1mhelp\x1b[0m\x1b[38;5;245m for commands, \x1b[1mtui\x1b[0m\x1b[38;5;245m for visual mode.\x1b[0m"
        );
    } else {
        println!("  Interactive mode. Type 'help' for commands, 'tui' for visual mode.");
    }
    println!();
}

pub fn print_interactive_help() {
    let is_color = std::io::stdout().is_terminal();

    if is_color {
        let d = "\x1b[38;5;245m";
        let c = "\x1b[38;5;51m";
        let b = "\x1b[1m";
        let r = "\x1b[0m";
        let g = "\x1b[38;5;77m";
        let y = "\x1b[38;5;220m";
        let s = "\x1b[38;5;240m\u{2502}\x1b[0m";
        let line = "\x1b[38;5;245m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m";

        println!();
        println!("  {b}{c}Interactive Commands{r}");
        println!("  {line}");

        println!("  {y}Query{r}");
        println!("  {g}recent{r} {d}[--limit N]{r}                {s} Show recent events");
        println!("  {g}search{r} {d}<query>{r}                   {s} Search events");
        println!("  {g}show{r} {d}<id>{r}                        {s} Full event details");
        println!("  {g}context{r} {d}[--limit N]{r}               {s} AI context summary");
        println!("  {g}stats{r}                             {s} Database statistics");

        println!("  {line}");
        println!("  {y}Modes{r}");
        println!("  {g}tui{r}                                {s} Switch to visual TUI dashboard");
        println!("  {g}clear{r}                              {s} Clear screen");
        println!("  {g}exit{r} / {g}quit{r}                       {s} Exit application");
        println!();
    } else {
        println!(
            "\n  Interactive Commands\n  ──────────────────────────────────────────────────\n  recent, search, show, context, stats, tui, clear, exit\n"
        );
    }
}

pub fn print_prompt() {
    use std::io::Write;
    let is_color = std::io::stdout().is_terminal();
    if is_color {
        print!("  \x1b[38;5;51m\u{276f}\x1b[0m\x1b[1m traz \x1b[0m");
    } else {
        print!("  > traz ");
    }
    std::io::stdout().flush().ok();
}

pub fn print_farewell() {
    println!("\n  Session ended. Your traces are safe.\n");
}
