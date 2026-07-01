use anyhow::Result;
use std::sync::Arc;
use traz_core::Event;
use traz_db::Db;

/// Colorize a single line of Cuby ASCII art based on the mood.
pub fn colorize_ascii_line(
    line_idx: usize,
    expression: &str,
    cuby_color: &str,
    reset: &str,
) -> String {
    // Let's get nice colors from display
    #[allow(non_snake_case)]
    let (_RESET, BOLD, DIM, _CYAN, _GREEN, _YELLOW, _MAGENTA, _BLUE) = crate::display::get_colors();

    let screen_color = DIM;
    let eye_color = BOLD;

    match line_idx {
        0 => {
            format!("  {cuby_color}▄▄▄▄▄▄▄▄▄▄{reset}")
        }
        1 => {
            format!(" {cuby_color}█{reset} {screen_color}┌──────┐{reset} {cuby_color}█{reset}")
        }
        2 => {
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

            format!(
                " {cuby_color}█{reset} {screen_color}│{reset} {eye_color}{eyes}{reset} {screen_color}│{reset} {cuby_color}█{reset}"
            )
        }
        3 => {
            format!(" {cuby_color}█{reset} {screen_color}└──────┘{reset} {cuby_color}█{reset}")
        }
        4 => {
            format!("  {cuby_color}▀▀▀▀▀▀▀▀▀▀{reset}")
        }
        5 => {
            format!("    {cuby_color}▀{reset}    {cuby_color}▀{reset}")
        }
        _ => "".to_string(),
    }
}

/// Print Cuby side-by-side with a set of text lines.
pub fn print_cuby_side_by_side(expression: &str, text_lines: &[String]) {
    let max_lines = std::cmp::max(6, text_lines.len());

    #[allow(non_snake_case)]
    let (RESET, _BOLD, _DIM, CYAN, _GREEN, YELLOW, MAGENTA, BLUE) = crate::display::get_colors();

    // Map expression to an awesome color!
    let cuby_color = match expression {
        "happy" | "curious" | "excited" | "starstruck" => CYAN,
        "sad" | "sleepy" | "crying" | "overwhelmed" => BLUE,
        "angry" | "scowling" | "error" | "offline" => MAGENTA,
        "dizzy" | "confused" | "premium" | "monetized" => YELLOW,
        "chill" | "content" => CYAN,
        _ => BLUE,
    };

    println!();
    for i in 0..max_lines {
        let colored_ascii = if i < 6 {
            colorize_ascii_line(i, expression, cuby_color, RESET)
        } else {
            "            ".to_string()
        };

        let text_line = if i < text_lines.len() {
            &text_lines[i]
        } else {
            ""
        };

        if !text_line.is_empty() {
            println!("  {}  {}", colored_ascii, text_line);
        } else if i < 6 {
            println!("  {}", colored_ascii);
        }
    }
    println!();
}

/// Handles the `traz cuby` subcommands.
pub async fn handle_cuby_command(subcommand: &str, args: &[String], db: Arc<Db>) -> Result<()> {
    #[allow(non_snake_case)]
    let (RESET, BOLD, DIM, CYAN, _GREEN, _YELLOW, _MAGENTA, _BLUE) = crate::display::get_colors();

    match subcommand {
        "status" | "stats" | "" => {
            let count = db.count_events().await?;
            let (mood, message) = if count == 0 {
                (
                    "sad",
                    "My memory is completely empty! Feed me some trace treats! 😢".to_string(),
                )
            } else if count <= 10 {
                (
                    "happy",
                    format!(
                        "I remember {} things! My brain is starting to spark! ⚡",
                        count
                    ),
                )
            } else if count <= 50 {
                (
                    "excited",
                    format!(
                        "Nom nom! I remember {} events! I'm getting so smart! 🌟",
                        count
                    ),
                )
            } else {
                (
                    "chill",
                    format!(
                        "Ah, absolute peace. I hold {} memories in my vault! 🧘",
                        count
                    ),
                )
            };

            let lines = vec![
                format!("{}── Cuby's Status ──{}", BOLD, RESET),
                format!("{}Memory Vault:{} {} events stored", DIM, RESET, count),
                format!("{}Current Mood:{} {}", DIM, RESET, mood.to_uppercase()),
                format!("{}Database:    {} {}", DIM, RESET, db.path().display()),
                "".to_string(),
                format!("{}{}{}", BOLD, message, RESET),
            ];

            print_cuby_side_by_side(mood, &lines);
        }

        "ask" | "query" | "search" => {
            if args.is_empty() {
                let lines = vec![
                    format!("{}Confused!{} 🌀", BOLD, RESET),
                    format!("You need to tell me what to search for!"),
                    format!("Example: {}traz cuby ask \"memory leak\"{}", CYAN, RESET),
                ];
                print_cuby_side_by_side("dizzy", &lines);
                return Ok(());
            }

            let query = args.join(" ");
            let filters = traz_db::SearchFilters::default();
            let results = db.search_events(&query, &filters, 5).await?;

            if results.is_empty() {
                let lines = vec![
                    format!("{}My brain is blank!{} 🧠❌", BOLD, RESET),
                    format!("I couldn't find any memories matching '{}'.", query),
                    format!("Are you sure you fed me that trace? Try another word!"),
                ];
                print_cuby_side_by_side("dizzy", &lines);
            } else {
                let mut lines = vec![
                    format!(
                        "{}*Sniffs context* I found {} matching memories!{}",
                        BOLD,
                        results.len(),
                        RESET
                    ),
                    "".to_string(),
                ];

                for (idx, event) in results.iter().enumerate() {
                    let rel = crate::display::relative_time(&event.timestamp);
                    let icon = crate::display::type_icon(&event.event_type);
                    lines.push(format!(
                        " {}. {} {} {}  {}[{} · {}]{}",
                        idx + 1,
                        icon,
                        BOLD,
                        event.title,
                        DIM,
                        event.tool,
                        rel,
                        RESET
                    ));
                }

                print_cuby_side_by_side("suspicious", &lines);
            }
        }

        "feed" | "eat" | "log" | "add" => {
            if args.is_empty() {
                let lines = vec![
                    format!("{}Wait! What are we eating?{} 🍕", BOLD, RESET),
                    format!("Please specify a memory title to feed me!"),
                    format!(
                        "Example: {}traz cuby feed \"Fixed login database bug\"{}",
                        CYAN, RESET
                    ),
                ];
                print_cuby_side_by_side("derp", &lines);
                return Ok(());
            }

            let title = args.join(" ");
            let branch = traz_integrations::git::get_current_branch_normalized();
            let event = Event {
                id: None,
                uuid: uuid::Uuid::new_v4().to_string(),
                tool: "cuby".to_string(),
                event_type: "decision".to_string(),
                title: title.clone(),
                summary: Some("Developer fed this memory directly to Cuby".to_string()),
                files: None,
                metadata: None,
                tags: Some(vec!["fed".to_string(), "cuby".to_string()]),
                session_id: None,
                diff: None,
                branch_name: branch,
                parent_event_id: None,
                is_checkpoint: None,
                agent_id: None,
                timestamp: chrono::Utc::now(),
                created_at: Some(chrono::Utc::now()),
            };

            let id = db.insert_event(&event).await?;
            let total = db.count_events().await?;

            let lines = vec![
                format!("{}*NOM NOM NOM* Delicious!{} 😋", BOLD, RESET),
                format!("I swallowed a new memory #{}! {}", id, title),
                format!("My vault now contains {} memories!", total),
                format!("Thank you for the delicious context!"),
            ];

            print_cuby_side_by_side("excited", &lines);
        }

        "talk" | "chat" | "quote" | "speak" => {
            let recent = db.get_recent_events(1).await?;
            if recent.is_empty() {
                let lines = vec![
                    format!("{}Zzz... *snore*...{}", BOLD, RESET),
                    format!("My memory is completely empty!"),
                    format!("Feed me some code events so I have something to talk about!"),
                ];
                print_cuby_side_by_side("asleep", &lines);
            } else {
                let event = &recent[0];
                let rel = crate::display::relative_time(&event.timestamp);

                let (mood, commentary) = match event.event_type.as_str() {
                    "bug_fix" => (
                        "happy",
                        format!(
                            "Aha! I saw you fixed a bug: '{}' ({}). Squash those bugs! 🐛💨",
                            event.title, rel
                        ),
                    ),
                    "feature" => (
                        "excited",
                        format!(
                            "Ooh! You added a shiny new feature: '{}' ({}). You're brilliant! ✨🚀",
                            event.title, rel
                        ),
                    ),
                    "refactor" => (
                        "focused",
                        format!(
                            "Cleaning house? You refactored: '{}' ({}). I love fresh, neat code! 🧹💻",
                            event.title, rel
                        ),
                    ),
                    "decision" => (
                        "chill",
                        format!(
                            "A wise choice was logged: '{}' ({}). Noted in my vault forever!",
                            event.title, rel
                        ),
                    ),
                    _ => (
                        "neutral",
                        format!(
                            "Hmm, a new trace: '{}' ({}) using {}. Keep up the great work!",
                            event.title, rel, event.tool
                        ),
                    ),
                };

                let lines = vec![
                    format!("{}Cuby says:{}", BOLD, RESET),
                    "".to_string(),
                    commentary,
                    "".to_string(),
                    format!(
                        "{}Memory ID: #{} · Tool: {}{}",
                        DIM,
                        event.id.unwrap_or(0),
                        event.tool,
                        RESET
                    ),
                ];

                print_cuby_side_by_side(mood, &lines);
            }
        }

        "mood" | "expression" | "face" => {
            if args.is_empty() {
                let lines = vec![
                    format!("{}Which mood?{} 🤔", BOLD, RESET),
                    format!("Tell me which mood to show! Available moods:"),
                    format!("  neutral, happy, sad, angry, surprised, wink, suspicious,"),
                    format!(
                        "  error, excited, dizzy, asleep, crying, focused, derp, chill, premium"
                    ),
                ];
                print_cuby_side_by_side("curious", &lines);
                return Ok(());
            }

            let requested_mood = args[0].to_lowercase();
            let quote = match requested_mood.as_str() {
                "neutral" | "default" => "Just chilling. Ready to remember everything!",
                "happy" | "curious" => {
                    "Today is a great day to write some code! What are we building?"
                }
                "sad" | "sleepy" => "Need... more... coffee... or maybe some trace events...",
                "angry" | "scowling" => {
                    "Who broke the build? Show them to me! I will scowl at them!"
                }
                "surprised" | "shocked" => "Whoa! Did you see that compile speed? Zoom!",
                "wink" | "mischievous" => {
                    "I know a secret... but I won't tell unless you write clean code."
                }
                "suspicious" | "processing" => {
                    "Hmm... is that a bug I smell? Or just a very creative feature?"
                }
                "error" | "offline" => "Core dump! Brain overflow! Just kidding, I'm just offline.",
                "excited" | "starstruck" => {
                    "Oh my gosh! You are writing beautiful code! Keep going!"
                }
                "dizzy" | "confused" => "Wait, what did we just do? Which branch are we on?",
                "asleep" | "idle" => "Zzz... 101010... sleeping on the job...",
                "crying" | "overwhelmed" => "Too many conflicts! My merge is broken! *Sob*",
                "focused" | "determined" => "Laser focus engaged. Let's squash some bugs.",
                "derp" | "buggy" => "I am a potato. Beep boop potato.",
                "chill" | "content" => "Code is compiling, tests are passing. Life is good.",
                "premium" | "monetized" => "To unlock advanced memories, please insert 0.0001 BTC.",
                _ => "",
            };

            if quote.is_empty() {
                let lines = vec![
                    format!("{}Invalid mood: '{}'{} 😵", BOLD, requested_mood, RESET),
                    format!(
                        "Valid moods: neutral, happy, sad, angry, surprised, wink, suspicious,"
                    ),
                    format!(
                        "             error, excited, dizzy, asleep, crying, focused, derp, chill, premium"
                    ),
                ];
                print_cuby_side_by_side("derp", &lines);
            } else {
                let lines = vec![
                    format!(
                        "{}Cuby feels: {}{}",
                        BOLD,
                        requested_mood.to_uppercase(),
                        RESET
                    ),
                    "".to_string(),
                    format!("\"{}\"", quote),
                ];
                print_cuby_side_by_side(&requested_mood, &lines);
            }
        }

        "play" | "game" | "tamagotchi" => {
            play_game(db).await?;
        }

        "dance" | "disco" => {
            play_dance_animation(80, 20, 50)?;
        }

        "sing" | "song" | "poem" => {
            let lines = vec![
                format!("{}Cuby clears its throat and sings:{} 🎶", BOLD, RESET),
                "".to_string(),
                format!("\"{}\"", get_random_song()),
            ];
            print_cuby_side_by_side("happy", &lines);
        }

        "pet" | "pat" | "purr" => {
            let pet_responses = [
                "*purrs softly* \"Oh yes, that's the spot! Happy developer, happy pet!\" 🐈",
                "*happy beep* \"Your cursor is so warm! Thank you for the head pats!\" 🥰",
                "*wiggles happily* \"I promise to remember every single bug fix for you!\" ⚡",
                "*nuzzles your terminal* \"Mmm, delicious context. Let's compile something!\" 🚀",
                "*rolls over* \"Beep boop! You are doing an amazing job today! Keep going!\" 🌟",
            ];
            let idx = (chrono::Utc::now().timestamp_subsec_millis() as usize) % pet_responses.len();
            let response = pet_responses[idx];

            let lines = vec![
                format!("{}Petting Cuby...{} 👋", BOLD, RESET),
                "".to_string(),
                response.to_string(),
            ];
            print_cuby_side_by_side("happy", &lines);
        }

        _ => {
            let lines = vec![
                format!("{}Beep? I don't know that command!{} ❓", BOLD, RESET),
                format!("Here are the commands I understand:"),
                format!(
                    "  {}cuby status{}       - Check my mood and memory status",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby play{}         - Open the interactive Tamagotchi pet game!",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby dance{}        - Watch Cuby perform a cute terminal dance!",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby sing{}         - Let Cuby sing you a developer song!",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby pet{}          - Pet Cuby for a happy reaction!",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby ask <query>{} - Search my memory vault for context",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby feed <text>{} - Feed me a new memory treat",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby talk{}         - Let me comment on your recent code trace",
                    CYAN, RESET
                ),
                format!(
                    "  {}cuby mood <mood>{} - Force me into any of my 16 expressions!",
                    CYAN, RESET
                ),
            ];
            print_cuby_side_by_side("dizzy", &lines);
        }
    }

    Ok(())
}

fn draw_bar(percentage: u32) -> String {
    let filled = (percentage / 10) as usize;
    let empty = 10 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn get_status_commentary(happiness: u32, hunger: u32, brain_power: u32) -> &'static str {
    if hunger > 70 {
        "I'm starving! Feed me some delicious traces! 🍕"
    } else if happiness < 40 {
        "I'm feeling a bit lonely... Could you pet me? 🥺"
    } else if brain_power > 80 {
        "My brain is pulsing with engineering knowledge! 🧠⚡"
    } else if happiness > 80 {
        "Purrrr... I'm so happy coding with you! 🥰"
    } else {
        "I'm ready! Let's write some beautiful software!"
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

fn clear_terminal() {
    // Clear terminal screen and scrollback buffer using the system clear command.
    // If it fails, fallback to ANSI escape codes.
    if std::process::Command::new("clear").status().is_err() {
        print!("\x1b[2J\x1b[H");
    }
}

fn render_game_screen(expression: &str, happiness: u32, hunger: u32, brain_power: u32) {
    #[allow(non_snake_case)]
    let (RESET, BOLD, DIM, _CYAN, GREEN, _YELLOW, _MAGENTA, _BLUE) = crate::display::get_colors();

    clear_terminal();

    let happy_bar = draw_bar(happiness);
    let hunger_bar = draw_bar(hunger);
    let brain_bar = draw_bar(brain_power);

    let lines = vec![
        format!("{}── Play with Cuby! ──{}", BOLD, RESET),
        format!("{}Happiness:{} {} {}%", DIM, RESET, happy_bar, happiness),
        format!("{}Hunger:   {} {} {}%", DIM, RESET, hunger_bar, hunger),
        format!("{}Brain IQ: {} {} {}%", DIM, RESET, brain_bar, brain_power),
        "".to_string(),
        format!(
            "\"{}\"",
            get_status_commentary(happiness, hunger, brain_power)
        ),
    ];

    print_cuby_side_by_side(expression, &lines);

    println!("  {}What should we do?{}", BOLD, RESET);
    println!("  {}1.{} Pet Cuby 👋", GREEN, RESET);
    println!("  {}2.{} Feed a Context Treat 🍕", GREEN, RESET);
    println!("  {}3.{} Sing a Developer Song 🎶", GREEN, RESET);
    println!("  {}4.{} Let's Dance! 💃", GREEN, RESET);
    println!("  {}5.{} Exit Game 🚪", GREEN, RESET);
}

pub fn play_dance_animation(happiness: u32, hunger: u32, brain_power: u32) -> Result<()> {
    #[allow(non_snake_case)]
    let (_RESET, _BOLD, _DIM, CYAN, _GREEN, _YELLOW, _MAGENTA, BLUE) = crate::display::get_colors();

    let dance_frames = vec![
        ("wink", CYAN),
        ("wink_right", BLUE),
        ("happy", CYAN),
        ("excited", BLUE),
        ("wink", CYAN),
        ("wink_right", BLUE),
        ("happy", CYAN),
        ("excited", BLUE),
    ];

    for (mood, _) in &dance_frames {
        render_game_screen(mood, happiness, hunger, brain_power);
        use std::io::Write;
        std::io::stdout().flush().ok();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Ok(())
}

pub async fn play_game(db: Arc<Db>) -> Result<()> {
    traz_tui::cuby_tui::run_cuby_game(db.path().to_path_buf()).await?;
    Ok(())
}
