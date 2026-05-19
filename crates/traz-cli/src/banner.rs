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
    //
    // Each line gets a progressively shifting 256-color code
    // to create a vertical gradient effect.
    let art: &[(&str, &str)] = &[
        ("\x1b[38;5;51m",  r#"          ████████╗██████╗   █████╗  ███████╗"#),
        ("\x1b[38;5;45m",  r#"          ╚══██╔══╝██╔══██╗ ██╔══██╗ ╚══███╔╝"#),
        ("\x1b[38;5;39m",  r#"             ██║   ██████╔╝ ███████║   ███╔╝ "#),
        ("\x1b[38;5;33m",  r#"             ██║   ██╔══██╗ ██╔══██║  ███╔╝  "#),
        ("\x1b[38;5;63m",  r#"             ██║   ██║  ██║ ██║  ██║ ███████╗"#),
        ("\x1b[38;5;99m",  r#"             ╚═╝   ╚═╝  ╚═╝ ╚═╝  ╚═╝ ╚══════╝"#),
    ];

    println!();
    for (color, line) in art {
        println!("{}{}\x1b[0m", color, line);
    }

    // Accent line
    println!(
        "\n  \x1b[38;5;240m─────────────────────────────────────────────────────\x1b[0m"
    );

    // Tagline
    println!(
        "  \x1b[38;5;245m  developer memory for AI-native workflows\x1b[0m"
    );

    // Version + status badges
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "  \x1b[38;5;240m──────────────────────────────────────────────────────\x1b[0m"
    );
    println!(
        "   \x1b[38;5;51m◆\x1b[0m \x1b[1mv{}\x1b[0m  \x1b[38;5;240m│\x1b[0m  \x1b[38;5;77m●\x1b[0m local-first  \x1b[38;5;240m│\x1b[0m  \x1b[38;5;77m●\x1b[0m zero-cloud",
        version
    );
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

/// Print the welcome message shown when entering interactive mode
pub fn print_interactive_welcome() {
    let is_color = std::io::stdout().is_terminal();

    if is_color {
        println!(
            "  \x1b[38;5;245mInteractive mode. Type \x1b[1mhelp\x1b[0m\x1b[38;5;245m for commands, \x1b[1mexit\x1b[0m\x1b[38;5;245m to quit.\x1b[0m"
        );
    } else {
        println!("  Interactive mode. Type 'help' for commands, 'exit' to quit.");
    }
    println!();
}

/// Print the interactive help menu
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
        println!("  {b}{c}Available Commands{r}");
        println!("  {line}");

        println!("  {y}Query{r}");
        println!("  {g}recent{r} {d}[--limit N] [--tool T]{r}       {s} Show recent events");
        println!("  {g}timeline{r} {d}[--limit N]{r}                {s} Chronological view");
        println!("  {g}search{r} {d}<query> [--tool T]{r}           {s} Search events");
        println!("  {g}show{r} {d}<id> [--json]{r}                  {s} Full event details");
        println!("  {g}context{r} {d}[--limit N] [--json]{r}        {s} AI context summary");
        println!("  {g}stats{r} {d}[--json]{r}                      {s} Database statistics");

        println!("  {line}");
        println!("  {y}Write{r}");
        println!("  {g}add{r} {d}--tool T --type E --title T{r}     {s} Add an event");
        println!("  {g}log{r} {d}<message>{r}                       {s} Quick log entry");
        println!("  {g}capture{r}                              {s} Capture latest commit");
        println!("  {g}delete{r} {d}<id>{r}                         {s} Delete event");
        println!("  {g}undo{r}                                 {s} Undo last event");
        println!("  {g}rewind{r} {d}<id>{r}                         {s} Rewind to checkpoint");

        println!("  {line}");
        println!("  {y}Data{r}");
        println!("  {g}diff{r} {d}<id>{r}                           {s} Show event diff");
        println!("  {g}export{r}                               {s} Export as JSON");
        println!("  {g}import{r}                               {s} Import from JSON stdin");

        println!("  {line}");
        println!("  {g}clear{r}                                {s} Clear screen");
        println!("  {g}banner{r}                               {s} Show banner");
        println!("  {g}help{r}                                 {s} Show this help");
        println!("  {g}exit{r} / {g}quit{r} / {g}Ctrl+C{r}                 {s} Exit");
        println!();
    } else {
        println!();
        println!("  Available Commands");
        println!("  ──────────────────────────────────────────────────");
        println!("  Query");
        println!("  recent [--limit N] [--tool T]       | Show recent events");
        println!("  timeline [--limit N]                | Chronological view");
        println!("  search <query> [--tool T]           | Search events");
        println!("  show <id> [--json]                  | Full event details");
        println!("  context [--limit N] [--json]        | AI context summary");
        println!("  stats [--json]                      | Database statistics");
        println!("  ──────────────────────────────────────────────────");
        println!("  Write");
        println!("  add --tool T --type E --title T     | Add an event");
        println!("  log <message>                       | Quick log entry");
        println!("  capture                             | Capture latest commit");
        println!("  delete <id>                         | Delete event");
        println!("  undo                                | Undo last event");
        println!("  rewind <id>                         | Rewind to checkpoint");
        println!("  ──────────────────────────────────────────────────");
        println!("  Data");
        println!("  diff <id>                           | Show event diff");
        println!("  export                              | Export as JSON");
        println!("  import                              | Import from JSON stdin");
        println!("  ──────────────────────────────────────────────────");
        println!("  clear                               | Clear screen");
        println!("  banner                              | Show banner");
        println!("  help                                | Show this help");
        println!("  exit / quit / Ctrl+C                | Exit");
        println!();
    }
}

/// Print the `❯ traz ` prompt (without newline)
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

/// Print a farewell message
pub fn print_farewell() {
    let is_color = std::io::stdout().is_terminal();
    if is_color {
        println!(
            "\n  \x1b[38;5;245m\u{2726} Session ended. Your traces are safe.\x1b[0m\n"
        );
    } else {
        println!("\n  Session ended. Your traces are safe.\n");
    }
}
