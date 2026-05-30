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
            "  \x1b[38;5;245mInteractive mode. Type \x1b[1mhelp\x1b[0m\x1b[38;5;245m for commands, \x1b[1mtui\x1b[0m\x1b[38;5;245m for visual mode, \x1b[1mcuby\x1b[0m\x1b[38;5;245m for pet mode.\x1b[0m"
        );
    } else {
        println!(
            "  Interactive mode. Type 'help' for commands, 'tui' for visual mode, 'cuby' for pet mode."
        );
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
        let line = "\x1b[38;5;245m\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\x1b[0m";

        println!();
        println!("  {b}{c}Interactive Commands{r}");
        println!("  {line}");

        println!("  {y}Query{r}");
        println!("  {g}recent{r} {d}[--limit N]{r}                {s} Show recent events");
        println!("  {g}timeline{r} {d}[--limit N]{r}              {s} Show chronological timeline");
        println!("  {g}search{r} {d}<query>{r}                   {s} Search events");
        println!("  {g}show{r} {d}<id>{r}                        {s} Full event details");
        println!("  {g}diff{r} {d}<id>{r}                        {s} Show code diff for event");
        println!("  {g}context{r} {d}[--limit N]{r}               {s} AI context summary");
        println!("  {g}stats{r}                             {s} Database statistics");
        println!("  {g}status{r}                            {s} Current system status");
        println!(
            "  {g}cuby{r} {d}[subcmd]{r}                      {s} Talk to Cuby, the context pet"
        );

        println!("  {line}");
        println!("  {y}Mutation{r}");
        println!("  {g}log{r} {d}<msg>{r}                        {s} Log a manual event shorthand");
        println!("  {g}add{r} {d}[opts]{r}                       {s} Add a detailed event");
        println!("  {g}capture{r}                          {s} Capture latest git commit");
        println!("  {g}undo{r}                             {s} Delete the most recent event");
        println!("  {g}delete{r} {d}<id>{r}                      {s} Delete an event by ID");
        println!("  {g}rewind{r} {d}<id>{r}                      {s} Delete all events after ID");
        println!("  {g}compress{r} {d}--summary <text>{r}        {s} Compress old events");

        println!("  {line}");
        println!("  {y}System{r}");
        println!("  {g}setup{r} {d}<tool>{r}                     {s} Show integration setup steps");
        println!("  {g}doctor{r}                            {s} Troubleshoot installation");
        println!("  {g}import{r} / {g}export{r}                   {s} Import/Export events (JSON)");

        println!("  {line}");
        println!("  {y}Modes{r}");
        println!("  {g}tui{r}                                {s} Switch to visual TUI dashboard");
        println!("  {g}clear{r}                              {s} Clear screen");
        println!("  {g}/exit{r} / {g}/quit{r}                     {s} Exit application");
        println!();
    } else {
        println!(
            "\n  Interactive Commands\n  ──────────────────────────────────────────────────────────\n  Query:   recent, timeline, search, show, diff, context, stats, status, cuby\n  Mutate:  log, add, capture, undo, delete, rewind, compress\n  System:  setup, doctor, import, export\n  Modes:   tui, clear, /exit\n"
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
