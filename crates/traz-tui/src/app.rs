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
