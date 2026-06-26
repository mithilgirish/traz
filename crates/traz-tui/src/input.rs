use crossterm::event::{KeyCode, KeyEvent};
use std::cmp::min;

use crate::app::{App, AppMode, ConfirmAction};

/// Process terminal keyboard input. Returns `Ok(true)` if the application should exit.
pub fn handle_input(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match &app.mode {
        AppMode::List => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.selected = app.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') if !app.events.is_empty() => {
                app.selected = min(app.selected + 1, app.events.len() - 1);
            }
            KeyCode::Enter => {
                if !app.events.is_empty()
                    && let Some(id) = app.events[app.selected].id
                {
                    app.scroll_offset = 0;
                    app.mode = AppMode::Detail(id);
                }
            }
            KeyCode::Char('d') => {
                if !app.events.is_empty()
                    && let Some(id) = app.events[app.selected].id
                {
                    app.scroll_offset = 0;
                    app.mode = AppMode::Diff(id);
                }
            }
            KeyCode::Char('u') => {
                if !app.events.is_empty()
                    && let Some(id) = app.events[app.selected].id
                {
                    app.previous_mode = Some(AppMode::List);
                    app.mode = AppMode::Confirm(ConfirmAction::Undo(id));
                }
            }
            KeyCode::Char('r') => {
                if !app.events.is_empty()
                    && let Some(id) = app.events[app.selected].id
                {
                    app.previous_mode = Some(AppMode::List);
                    app.mode = AppMode::Confirm(ConfirmAction::Rewind(id));
                }
            }
            KeyCode::Char('c') => {
                app.previous_mode = Some(AppMode::List);
                app.mode = AppMode::Confirm(ConfirmAction::Compress);
            }
            KeyCode::Char('/') => {
                app.mode = AppMode::Search;
                app.search_query.clear();
                app.filter_events();
            }
            KeyCode::Char('s') | KeyCode::Char(',') => {
                app.previous_mode = Some(AppMode::List);
                app.mode = AppMode::Settings;
                app.selected_setting = 0;
            }
            _ => {}
        },
        AppMode::Detail(id) => {
            let current_id = *id;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.scroll_offset = 0;
                    app.mode = AppMode::List;
                }
                KeyCode::Char('d') => {
                    app.scroll_offset = 0;
                    app.mode = AppMode::Diff(current_id);
                }
                KeyCode::Char('u') => {
                    app.previous_mode = Some(AppMode::Detail(current_id));
                    app.mode = AppMode::Confirm(ConfirmAction::Undo(current_id));
                }
                KeyCode::Char('r') => {
                    app.previous_mode = Some(AppMode::Detail(current_id));
                    app.mode = AppMode::Confirm(ConfirmAction::Rewind(current_id));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                }
                KeyCode::PageUp => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    app.scroll_offset = app.scroll_offset.saturating_add(10);
                }
                _ => {}
            }
        }
        AppMode::Diff(id) => {
            let current_id = *id;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.scroll_offset = 0;
                    app.mode = AppMode::Detail(current_id);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.scroll_offset = app.scroll_offset.saturating_add(1);
                }
                KeyCode::PageUp => {
                    app.scroll_offset = app.scroll_offset.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    app.scroll_offset = app.scroll_offset.saturating_add(10);
                }
                _ => {}
            }
        }
        AppMode::Search => match key.code {
            KeyCode::Esc => {
                app.search_query.clear();
                app.filter_events();
                app.mode = AppMode::List;
            }
            KeyCode::Enter => {
                app.mode = AppMode::List;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.filter_events();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.filter_events();
            }
            _ => {}
        },
        AppMode::Confirm(action) => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                match action {
                    ConfirmAction::Undo(id) => match app.db.delete_event(*id) {
                        Ok(true) => {
                            app.set_status("✓ Event deleted");
                        }
                        Ok(false) => {
                            app.set_status("✗ Event not found");
                        }
                        Err(e) => {
                            app.set_status(&format!("✗ {}", e));
                        }
                    },
                    ConfirmAction::Rewind(id) => match app.db.delete_events_after(*id) {
                        Ok(count) => {
                            app.set_status(&format!("✓ Rewound — {} events removed", count));
                        }
                        Err(e) => {
                            app.set_status(&format!("✗ {}", e));
                        }
                    },
                    ConfirmAction::Compress => {
                        match app
                            .db
                            .compress_events(14, "Interactive TUI compression".to_string())
                        {
                            Ok((count, _)) => {
                                if count > 0 {
                                    app.set_status(&format!(
                                        "✓ Compressed {} events into epoch",
                                        count
                                    ));
                                } else {
                                    app.set_status("✓ No events older than 14 days to compress");
                                }
                            }
                            Err(e) => {
                                app.set_status(&format!("✗ {}", e));
                            }
                        }
                    }
                }
                app.reload_events()?;
                app.mode = AppMode::List;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                // Return to the previous mode
                app.mode = app.previous_mode.clone().unwrap_or(AppMode::List);
            }
            _ => {}
        },
        AppMode::Settings => match key.code {
            KeyCode::Esc | KeyCode::Char('s') | KeyCode::Char('q') => {
                app.mode = app.previous_mode.clone().unwrap_or(AppMode::List);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.selected_setting = app.selected_setting.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.selected_setting = std::cmp::min(app.selected_setting + 1, 2);
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                match app.selected_setting {
                    0 => {
                        match app.theme_option {
                            crate::app::ThemeOption::Dark => {
                                app.theme_option = crate::app::ThemeOption::Light;
                                app.is_dark_mode = false;
                                app.set_status("✓ Switched theme to Catppuccin Light");
                            }
                            crate::app::ThemeOption::Light => {
                                app.theme_option = crate::app::ThemeOption::Custom;
                                app.is_dark_mode = true;

                                // Proactively write a custom theme template if it does not exist!
                                if !app.custom_theme_path.exists() {
                                    let template = serde_json::json!({
                                        "bg_dark": "#1a1b26",
                                        "panel_bg": "#1f2335",
                                        "sel_bg": "#33467c",
                                        "text_muted": "#565f89",
                                        "text_main": "#c0caf5",
                                        "blue": "#7aa2f7",
                                        "green": "#9ece6a",
                                        "pink": "#bb9af3",
                                        "yellow": "#e0af68",
                                        "red": "#f7768e"
                                    });
                                    if let Ok(pretty) = serde_json::to_string_pretty(&template) {
                                        let _ = std::fs::write(&app.custom_theme_path, pretty);
                                    }
                                }

                                app.set_status("✓ Switched theme to Custom (theme.json)");
                            }
                            crate::app::ThemeOption::Custom => {
                                app.theme_option = crate::app::ThemeOption::Dark;
                                app.is_dark_mode = true;
                                app.set_status("✓ Switched theme to Catppuccin Dark");
                            }
                        }
                    }
                    1 => {
                        app.show_timeline = !app.show_timeline;
                        let status = if app.show_timeline {
                            "Timeline tree enabled"
                        } else {
                            "Timeline tree disabled"
                        };
                        app.set_status(&format!("✓ {}", status));
                    }
                    2 => {
                        app.show_gutters = !app.show_gutters;
                        let status = if app.show_gutters {
                            "Line gutters enabled"
                        } else {
                            "Line gutters disabled"
                        };
                        app.set_status(&format!("✓ {}", status));
                    }
                    _ => {}
                }
            }
            _ => {}
        },
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, AppMode};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::fs;
    use std::time::SystemTime;
    use traz_db::Db;

    fn setup_test_app(test_name: &str) -> (App, std::path::PathBuf) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_dir =
            std::env::temp_dir().join(format!("traz_tui_input_test_{}_{}", test_name, ts));
        let _ = fs::create_dir_all(&unique_dir);
        let db_path = unique_dir.join("traz.db");
        let db = Db::open(&db_path).unwrap();

        let event1 = traz_core::Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "Title 1".to_string(),
            None,
            None,
            None,
        );
        let event2 = traz_core::Event::new(
            "aider".to_string(),
            "bug_fix".to_string(),
            "Title 2".to_string(),
            None,
            None,
            None,
        );

        let app = App::new(db, vec![event1, event2], unique_dir.join("theme.json"));
        (app, unique_dir)
    }

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    #[test]
    fn test_handle_input_exit() {
        let (mut app, test_dir) = setup_test_app("exit");

        // Esc or 'q' in List mode should exit
        assert!(handle_input(&mut app, press(KeyCode::Char('q'))).unwrap());
        assert!(handle_input(&mut app, press(KeyCode::Esc)).unwrap());

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_handle_input_navigation() {
        let (mut mut_app, test_dir) = setup_test_app("nav");

        // Starts at index 0
        assert_eq!(mut_app.selected, 0);

        // Press down
        handle_input(&mut mut_app, press(KeyCode::Char('j'))).unwrap();
        assert_eq!(mut_app.selected, 1);

        // Press down again (saturates at length - 1, which is 1)
        handle_input(&mut mut_app, press(KeyCode::Down)).unwrap();
        assert_eq!(mut_app.selected, 1);

        // Press up
        handle_input(&mut mut_app, press(KeyCode::Char('k'))).unwrap();
        assert_eq!(mut_app.selected, 0);

        // Press up again (saturates at 0)
        handle_input(&mut mut_app, press(KeyCode::Up)).unwrap();
        assert_eq!(mut_app.selected, 0);

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_handle_input_search_mode() {
        let (mut app, test_dir) = setup_test_app("search");

        // Press '/' to search
        handle_input(&mut app, press(KeyCode::Char('/'))).unwrap();
        assert_eq!(app.mode, AppMode::Search);
        assert_eq!(app.search_query, "");

        // Type characters
        handle_input(&mut app, press(KeyCode::Char('a'))).unwrap();
        assert_eq!(app.search_query, "a");

        handle_input(&mut app, press(KeyCode::Char('b'))).unwrap();
        assert_eq!(app.search_query, "ab");

        // Backspace
        handle_input(&mut app, press(KeyCode::Backspace)).unwrap();
        assert_eq!(app.search_query, "a");

        // Press Enter to confirm search query and return to list mode
        handle_input(&mut app, press(KeyCode::Enter)).unwrap();
        assert_eq!(app.mode, AppMode::List);
        assert_eq!(app.search_query, "a");

        // Esc clears search query and returns to list mode
        handle_input(&mut app, press(KeyCode::Char('/'))).unwrap();
        assert_eq!(app.mode, AppMode::Search);
        handle_input(&mut app, press(KeyCode::Esc)).unwrap();
        assert_eq!(app.mode, AppMode::List);
        assert_eq!(app.search_query, "");

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_handle_input_settings_mode() {
        let (mut app, test_dir) = setup_test_app("settings");

        // Press 's' to enter settings
        handle_input(&mut app, press(KeyCode::Char('s'))).unwrap();
        assert_eq!(app.mode, AppMode::Settings);
        assert_eq!(app.selected_setting, 0);

        // Select next setting
        handle_input(&mut app, press(KeyCode::Char('j'))).unwrap();
        assert_eq!(app.selected_setting, 1);

        // Press Esc to exit settings
        handle_input(&mut app, press(KeyCode::Esc)).unwrap();
        assert_eq!(app.mode, AppMode::List);

        let _ = fs::remove_dir_all(test_dir);
    }
}
