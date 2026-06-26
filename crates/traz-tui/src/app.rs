use std::time::Instant;
use traz_core::Event;
use traz_db::Db;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    Undo(i64),
    Rewind(i64),
    Compress,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeOption {
    Dark,
    Light,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    List,
    Detail(i64),
    Diff(i64),
    Search,
    Confirm(ConfirmAction),
    Settings,
}

pub struct App {
    pub events: Vec<Event>,
    pub all_events: Vec<Event>,
    pub search_scores: Vec<f32>, // Similarity scores for current events
    pub selected: usize,
    pub mode: AppMode,
    pub previous_mode: Option<AppMode>, // Track where to return after Confirm/Settings mode
    pub search_query: String,
    pub status_message: Option<String>,
    pub status_message_time: Option<Instant>,
    pub scroll_offset: usize,
    pub db: Db,

    // Aesthetic Preference Switches
    pub is_dark_mode: bool,
    pub theme_option: ThemeOption,
    pub custom_theme_path: std::path::PathBuf,
    pub show_timeline: bool,
    pub show_gutters: bool,
    pub selected_setting: usize, // Index for selected setting in settings modal (0 to 2)
}

impl App {
    pub fn new(db: Db, events: Vec<Event>, custom_theme_path: std::path::PathBuf) -> Self {
        Self {
            all_events: events.clone(),
            events,
            search_scores: Vec::new(),
            selected: 0,
            mode: AppMode::List,
            previous_mode: None,
            search_query: String::new(),
            status_message: None,
            status_message_time: None,
            scroll_offset: 0,
            db,
            is_dark_mode: true,
            theme_option: ThemeOption::Dark,
            custom_theme_path,
            show_timeline: true,
            show_gutters: true,
            selected_setting: 0,
        }
    }

    /// Update the current events list based on search.
    /// If embeddings are enabled, it performs a semantic search.
    /// Otherwise, it performs a keyword search in the database.
    pub fn filter_events(&mut self) {
        self.search_scores.clear();

        if self.search_query.is_empty() {
            self.events = self.all_events.clone();
            return;
        }

        match self
            .db
            .hybrid_search(&self.search_query, &traz_db::SearchFilters::default(), 50)
        {
            Ok(results) => {
                self.events = results.iter().map(|(e, _)| e.clone()).collect();
                self.search_scores = results.into_iter().map(|(_, s)| s).collect();
            }
            Err(e) => {
                self.set_status(&format!("Search error: {}", e));
            }
        }

        // Clamp selected index to ensure it is in bounds
        if self.events.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.events.len() {
            self.selected = self.events.len() - 1;
        }
    }

    /// Set a temporary status message that clears after 3 seconds
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some(msg.to_string());
        self.status_message_time = Some(Instant::now());
    }

    /// Clear the status message if 3 seconds have elapsed
    pub fn check_status_message(&mut self) {
        if let Some(time) = self.status_message_time
            && time.elapsed().as_secs() >= 3
        {
            self.status_message = None;
            self.status_message_time = None;
        }
    }

    /// Reload events from the database
    pub fn reload_events(&mut self) -> anyhow::Result<()> {
        let events = self.db.get_recent_events(100)?;
        self.all_events = events;
        self.filter_events();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    fn setup_test_db(test_name: &str) -> (Db, std::path::PathBuf) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_dir =
            std::env::temp_dir().join(format!("traz_tui_app_test_{}_{}", test_name, ts));
        let _ = fs::create_dir_all(&unique_dir);
        let db_path = unique_dir.join("traz.db");
        let db = Db::open(&db_path).unwrap();
        (db, unique_dir)
    }

    #[test]
    fn test_app_initialization() {
        let (db, test_dir) = setup_test_db("init");
        let custom_theme = test_dir.join("theme.json");

        let event1 = traz_core::Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "Title 1".to_string(),
            None,
            None,
            None,
        );
        let app = App::new(db, vec![event1.clone()], custom_theme.clone());

        assert_eq!(app.all_events.len(), 1);
        assert_eq!(app.events.len(), 1);
        assert_eq!(app.selected, 0);
        assert_eq!(app.mode, AppMode::List);
        assert_eq!(app.search_query, "");
        assert!(app.status_message.is_none());
        assert!(app.is_dark_mode);
        assert_eq!(app.theme_option, ThemeOption::Dark);
        assert_eq!(app.custom_theme_path, custom_theme);

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_app_status_message_lifecycle() {
        let (db, test_dir) = setup_test_db("status");
        let mut app = App::new(db, vec![], test_dir.join("theme.json"));

        app.set_status("Hello status");
        assert_eq!(app.status_message, Some("Hello status".to_string()));
        assert!(app.status_message_time.is_some());

        // Call check_status_message immediately - shouldn't clear yet
        app.check_status_message();
        assert_eq!(app.status_message, Some("Hello status".to_string()));

        // Simulate passage of time by mutating the status time backwards by 4 seconds
        app.status_message_time = Some(Instant::now() - Duration::from_secs(4));
        app.check_status_message();

        // Now it should be cleared
        assert!(app.status_message.is_none());
        assert!(app.status_message_time.is_none());

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_app_filtering_and_reloading() {
        let (db, test_dir) = setup_test_db("filter");

        // Insert events directly to DB so reloading pulls them
        let e1 = traz_core::Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "Compile error solved".to_string(),
            None,
            None,
            None,
        );
        let e2 = traz_core::Event::new(
            "aider".to_string(),
            "bug_fix".to_string(),
            "Unrelated item".to_string(),
            None,
            None,
            None,
        );
        db.insert_event(&e1).unwrap();
        db.insert_event(&e2).unwrap();

        let mut app = App::new(
            db,
            vec![e1.clone(), e2.clone()],
            test_dir.join("theme.json"),
        );

        // 1. Keyword filter
        app.search_query = "error".to_string();
        app.filter_events();
        assert_eq!(app.events.len(), 1);
        assert_eq!(app.events[0].title, "Compile error solved");

        // 2. Clear query
        app.search_query.clear();
        app.filter_events();
        assert_eq!(app.events.len(), 2);

        // 3. Reload from DB
        app.reload_events().unwrap();
        assert_eq!(app.all_events.len(), 2);

        let _ = fs::remove_dir_all(test_dir);
    }
}
