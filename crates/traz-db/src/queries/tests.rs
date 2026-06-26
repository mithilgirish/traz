#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::database::Db;
    use anyhow::Result;
    use rusqlite::Connection;
    use std::path::PathBuf;
    use std::sync::Mutex;
    use traz_core::Event;

    impl Db {
        fn migrate_for_test(&self) -> Result<()> {
            let conn = self.conn.lock().unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS events (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    uuid        TEXT,
                    tool        TEXT    NOT NULL,
                    type        TEXT    NOT NULL,
                    title       TEXT    NOT NULL,
                    summary     TEXT,
                    files       TEXT,
                    metadata    TEXT,
                    tags        TEXT,
                    session_id  TEXT,
                    diff        TEXT,
                    timestamp   TEXT    NOT NULL,
                    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                );",
            )?;
            Ok(())
        }
    }

    fn test_db() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        let db = Db {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
            config: traz_core::TrazConfig {
                db_path: PathBuf::from(":memory:"),
                api_port: 4000,
                embeddings_enabled: false,
                embeddings_model_path: None,
            },
        };
        db.migrate_for_test().unwrap();
        db
    }

    fn sample_event(tool: &str, event_type: &str, title: &str) -> Event {
        Event::new(
            tool.to_string(),
            event_type.to_string(),
            title.to_string(),
            Some(format!("Summary for {}", title)),
            Some(vec!["src/main.rs".to_string()]),
            None,
        )
    }

    #[test]
    fn test_insert_and_retrieve() {
        let db = test_db();
        let event = sample_event("cursor", "feature", "Added login page");
        let id = db.insert_event(&event).unwrap();
        assert!(id > 0);

        let retrieved = db.get_event(id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Added login page");
        assert_eq!(retrieved.tool, "cursor");
        assert_eq!(retrieved.event_type, "feature");
        assert!(retrieved.summary.is_some());
        assert!(retrieved.files.is_some());
    }

    #[test]
    fn test_delete_event() {
        let db = test_db();
        let event = sample_event("aider", "bug_fix", "Fix null pointer");
        let id = db.insert_event(&event).unwrap();

        assert!(db.delete_event(id).unwrap());
        assert!(!db.delete_event(id).unwrap()); // already deleted
        assert!(db.get_event(id).unwrap().is_none());
    }

    #[test]
    fn test_search() {
        let db = test_db();
        let e1 = Event::new("t1".into(), "f1".into(), "Find me".into(), None, None, None);
        let e2 = Event::new("t2".into(), "f2".into(), "Hide me".into(), None, None, None);
        let e3 = Event::new(
            "t3".into(),
            "f3".into(),
            "Auth issue".into(),
            Some("bug".into()),
            None,
            None,
        );

        db.insert_event(&e1).unwrap();
        db.insert_event(&e2).unwrap();
        db.insert_event(&e3).unwrap();

        let results = db
            .search_events("Find", &crate::queries::read::SearchFilters::default(), 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Find me");

        // Test multi-word search: 'Auth' in title and 'bug' in summary
        let results2 = db
            .search_events(
                "Auth bug",
                &crate::queries::read::SearchFilters::default(),
                10,
            )
            .unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].title, "Auth issue");
    }

    #[test]
    fn test_search_with_tool_filter() {
        let db = test_db();
        db.insert_event(&sample_event("cursor", "feature", "Auth module"))
            .unwrap();
        db.insert_event(&sample_event("claude", "feature", "Auth refactor"))
            .unwrap();

        let results = db
            .search_events(
                "Auth",
                &crate::queries::read::SearchFilters {
                    tool: Some("cursor"),
                    ..Default::default()
                },
                10,
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tool, "cursor");
    }

    #[test]
    fn test_timeline_order() {
        let db = test_db();
        db.insert_event(&sample_event("t1", "f", "First")).unwrap();
        db.insert_event(&sample_event("t2", "f", "Second")).unwrap();
        db.insert_event(&sample_event("t3", "f", "Third")).unwrap();

        let timeline = db.get_timeline(10).unwrap();
        assert_eq!(timeline.len(), 3);
        // Timeline is oldest-first
        assert_eq!(timeline[0].title, "First");
        assert_eq!(timeline[2].title, "Third");
    }

    #[test]
    fn test_recent_events_order() {
        let db = test_db();
        db.insert_event(&sample_event("t1", "f", "First")).unwrap();
        db.insert_event(&sample_event("t2", "f", "Second")).unwrap();

        let recent = db.get_recent_events(10).unwrap();
        // Recent is newest-first
        assert_eq!(recent[0].title, "Second");
        assert_eq!(recent[1].title, "First");
    }

    #[test]
    fn test_stats() {
        let db = test_db();
        db.insert_event(&sample_event("cursor", "feature", "A"))
            .unwrap();
        db.insert_event(&sample_event("cursor", "bug_fix", "B"))
            .unwrap();
        db.insert_event(&sample_event("claude", "refactor", "C"))
            .unwrap();

        let stats = db.get_stats().unwrap();
        assert!(!stats.is_empty());
        // cursor should have 2, claude should have 1
        let cursor_count = stats.iter().find(|(t, _)| t == "cursor").map(|(_, c)| *c);
        assert_eq!(cursor_count, Some(2));

        let count = db.count_events().unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_filtered_events() {
        let db = test_db();
        db.insert_event(&sample_event("cursor", "feature", "A"))
            .unwrap();
        db.insert_event(&sample_event("claude", "bug_fix", "B"))
            .unwrap();
        db.insert_event(&sample_event("cursor", "bug_fix", "C"))
            .unwrap();

        // Filter by tool
        let results = db
            .get_filtered_events(10, Some("cursor".into()), None, None, None)
            .unwrap();
        assert_eq!(results.len(), 2);

        // Filter by type
        let results = db
            .get_filtered_events(10, None, Some("bug_fix".into()), None, None)
            .unwrap();
        assert_eq!(results.len(), 2);

        // Filter by tool AND type
        let results = db
            .get_filtered_events(
                10,
                Some("cursor".into()),
                Some("bug_fix".into()),
                None,
                None,
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "C");
    }

    #[test]
    fn test_get_last_event_id() {
        let db = test_db();
        assert!(db.get_last_event_id().unwrap().is_none());

        db.insert_event(&sample_event("t", "f", "A")).unwrap();
        let id2 = db.insert_event(&sample_event("t", "f", "B")).unwrap();

        assert_eq!(db.get_last_event_id().unwrap(), Some(id2));
    }

    #[test]
    fn test_get_event_by_uuid() {
        let db = test_db();
        let event = sample_event("cursor", "feature", "UUID test");
        let uuid = event.uuid.clone();
        db.insert_event(&event).unwrap();

        let found = db.get_event_by_uuid(&uuid).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "UUID test");

        assert!(db.get_event_by_uuid("nonexistent").unwrap().is_none());
    }

    #[test]
    fn test_session_events() {
        let db = test_db();
        let e1 = sample_event("t", "f", "A").with_session("sess-1".into());
        let e2 = sample_event("t", "f", "B").with_session("sess-1".into());
        let e3 = sample_event("t", "f", "C").with_session("sess-2".into());

        db.insert_event(&e1).unwrap();
        db.insert_event(&e2).unwrap();
        db.insert_event(&e3).unwrap();

        let sess1 = db.get_session_events("sess-1", 10).unwrap();
        assert_eq!(sess1.len(), 2);

        let sessions = db.get_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_context_summary() {
        let db = test_db();
        db.insert_event(
            &sample_event("cursor", "feature", "Built auth").with_tags(vec!["security".into()]),
        )
        .unwrap();
        db.insert_event(&sample_event("claude", "bug_fix", "Fixed race"))
            .unwrap();

        let ctx = db.get_context_summary(None, 10).unwrap();
        assert!(ctx.contains("Engineering Context Summary"));
        assert!(ctx.contains("cursor"));
        assert!(ctx.contains("Built auth"));
        assert!(ctx.contains("Fixed race"));
        assert!(ctx.contains("#security"));
    }

    #[test]
    #[ignore = "Requires downloading embedding model"]
    fn test_context_summary_rag() {
        let conn = Connection::open_in_memory().unwrap();
        let db = Db {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
            config: traz_core::TrazConfig {
                db_path: PathBuf::from(":memory:"),
                api_port: 4000,
                embeddings_enabled: true, // Needed for semantic search
                embeddings_model_path: None,
            },
        };
        db.migrate().unwrap();

        db.insert_event(&sample_event(
            "cursor",
            "feature",
            "Built authentication system",
        ))
        .unwrap();
        db.insert_event(&sample_event(
            "claude",
            "bug_fix",
            "Fixed CSS layout issues",
        ))
        .unwrap();

        let ctx = db.get_context_summary(Some("auth login"), 10).unwrap();
        assert!(ctx.contains("Relevant Context (RAG"));
        assert!(ctx.contains("Built authentication system"));
        assert!(!ctx.contains("CSS layout"));
    }

    #[test]
    fn test_tags_and_metadata() {
        let db = test_db();
        let event = sample_event("t", "f", "Tagged")
            .with_tags(vec!["rust".into(), "perf".into()])
            .with_metadata(serde_json::json!({"key": "value"}));
        let id = db.insert_event(&event).unwrap();

        let retrieved = db.get_event(id).unwrap().unwrap();
        assert_eq!(retrieved.tags.unwrap(), vec!["rust", "perf"]);
        assert_eq!(retrieved.metadata.unwrap()["key"], "value");
    }

    #[test]
    fn test_diff_storage() {
        let db = test_db();
        let event =
            sample_event("t", "f", "With diff").with_diff("+added line\n-removed line".into());
        let id = db.insert_event(&event).unwrap();

        let retrieved = db.get_event(id).unwrap().unwrap();
        assert!(retrieved.diff.unwrap().contains("+added line"));
    }

    #[test]
    fn test_validation_rejects_empty_fields() {
        let db = test_db();
        let event = Event::new("".into(), "f".into(), "T".into(), None, None, None);
        assert!(db.insert_event(&event).is_err());

        let event = Event::new("t".into(), "".into(), "T".into(), None, None, None);
        assert!(db.insert_event(&event).is_err());

        let event = Event::new("t".into(), "f".into(), "  ".into(), None, None, None);
        assert!(db.insert_event(&event).is_err());
    }

    #[test]
    fn test_limit_capping() {
        let db = test_db();
        for i in 0..5 {
            db.insert_event(&sample_event("t", "f", &format!("Event {}", i)))
                .unwrap();
        }

        let events = db.get_recent_events(3).unwrap();
        assert_eq!(events.len(), 3);

        let events = db.get_recent_events(100).unwrap();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_count_events_after() {
        let db = test_db();
        let id1 = db.insert_event(&sample_event("t", "f", "1")).unwrap();
        let id2 = db.insert_event(&sample_event("t", "f", "2")).unwrap();
        let id3 = db.insert_event(&sample_event("t", "f", "3")).unwrap();

        assert_eq!(db.count_events_after(id1).unwrap(), 2);
        assert_eq!(db.count_events_after(id2).unwrap(), 1);
        assert_eq!(db.count_events_after(id3).unwrap(), 0);
    }

    #[test]
    #[ignore = "Requires downloading embedding model"]
    fn test_semantic_search() {
        let conn = Connection::open_in_memory().unwrap();
        let db = Db {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
            config: traz_core::TrazConfig {
                db_path: PathBuf::from(":memory:"),
                api_port: 4000,
                embeddings_enabled: true,
                embeddings_model_path: None,
            },
        };
        db.migrate().unwrap();

        let e1 = Event::new(
            "t1".into(),
            "f1".into(),
            "Authentication bug fix in backend".into(),
            Some("Fixed database reconnect during login session".into()),
            None,
            None,
        );
        let e2 = Event::new(
            "t2".into(),
            "f2".into(),
            "Frontend CSS layout redesign".into(),
            Some("Re-aligned flex box grid items to center".into()),
            None,
            None,
        );

        let id1 = db.insert_event(&e1).unwrap();
        let id2 = db.insert_event(&e2).unwrap();

        // Search for something related to auth
        let results = db.semantic_search("login database", 10).unwrap();

        // Assert we got results back
        assert!(!results.is_empty());
        // e1 should be ranked first because it contains "login" and "database reconnect"
        assert_eq!(results[0].0.id, Some(id1));

        // Search for something related to styling
        let results_css = db
            .semantic_search("css flexbox layout alignment", 10)
            .unwrap();
        assert!(!results_css.is_empty());
        assert_eq!(results_css[0].0.id, Some(id2));
    }

    #[test]
    fn test_db_concurrency() {
        use std::sync::Arc;
        use std::thread;

        // Create a real database file in a temp directory so separate connections
        // actually hit the filesystem concurrently
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_dir = std::env::temp_dir().join(format!("traz_db_concurrency_{}", ts));
        let _ = std::fs::create_dir_all(&unique_dir);
        let db_path = unique_dir.join("traz.db");

        // Initialize the DB file by opening it once
        {
            let _db = Db::open(&db_path).unwrap();
        }

        // Spawn multiple threads, each opening their own Connection to the same db_path,
        // and concurrently calling insert_event
        let db_path_arc = Arc::new(db_path.clone());
        let mut handles = vec![];

        for i in 0..10 {
            let path_clone = Arc::clone(&db_path_arc);
            let handle = thread::spawn(move || {
                let db = Db::open(&path_clone).unwrap();
                let event = sample_event("cursor", "feature", &format!("Thread {} event", i));
                db.insert_event(&event).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads to finish
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all 10 events were written successfully
        let db = Db::open(&db_path).unwrap();
        let count = db.count_events().unwrap();
        assert_eq!(count, 10);

        let _ = std::fs::remove_dir_all(unique_dir);
    }

    #[test]
    fn test_delete_events_after() {
        let db = test_db();
        let id1 = db
            .insert_event(&sample_event("cursor", "feature", "First event"))
            .unwrap();
        let id2 = db
            .insert_event(&sample_event("cursor", "feature", "Second event"))
            .unwrap();
        let id3 = db
            .insert_event(&sample_event("cursor", "feature", "Third event"))
            .unwrap();

        let affected = db.delete_events_after(id1).unwrap();
        assert_eq!(affected, 2);

        // Verify id1 still exists but id2 and id3 are gone
        assert!(db.get_event(id1).unwrap().is_some());
        assert!(db.get_event(id2).unwrap().is_none());
        assert!(db.get_event(id3).unwrap().is_none());
    }

    #[test]
    fn test_compress_events() {
        let db = test_db();

        // 1. Insert an old event (e.g. 5 days ago)
        let mut old_event = sample_event("cursor", "bug_fix", "Old bug fix");
        let five_days_ago = chrono::Utc::now() - chrono::Duration::days(5);
        old_event.timestamp = five_days_ago;
        let old_id = db.insert_event(&old_event).unwrap();

        // 2. Insert a new event (e.g. just now)
        let new_event = sample_event("cursor", "feature", "New feature");
        let new_id = db.insert_event(&new_event).unwrap();

        // 3. Compress events older than 3 days
        let (count, epoch_id) = db
            .compress_events(3, "Summary of older epoch".to_string())
            .unwrap();

        assert_eq!(count, 1);
        assert!(epoch_id > 0);

        // Verify old event was deleted
        assert!(db.get_event(old_id).unwrap().is_none());

        // Verify new event still exists
        assert!(db.get_event(new_id).unwrap().is_some());

        // Verify epoch event was created
        let epoch_event = db.get_event(epoch_id).unwrap().unwrap();
        assert_eq!(epoch_event.event_type, "epoch");
        assert_eq!(
            epoch_event.summary,
            Some("Summary of older epoch".to_string())
        );
    }

    #[test]
    fn test_hybrid_search_without_embeddings() {
        let db = test_db();
        // Since test_db has embeddings disabled by default config,
        // hybrid_search should fallback to purely keyword search results
        let _id1 = db
            .insert_event(&sample_event("claude", "decision", "Rust is great"))
            .unwrap();
        let id2 = db
            .insert_event(&sample_event("claude", "bug_fix", "Fix compilation error"))
            .unwrap();

        let results = db
            .hybrid_search(
                "compilation",
                &crate::queries::read::SearchFilters::default(),
                10,
            )
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.id, Some(id2));
    }

    #[test]
    fn test_get_context_optimized_budget() {
        let db = test_db();
        db.insert_event(&sample_event("cursor", "feature", "Event A"))
            .unwrap();
        db.insert_event(&sample_event("cursor", "feature", "Event B"))
            .unwrap();
        db.insert_event(&sample_event("cursor", "feature", "Event C"))
            .unwrap();

        // 1. Markdown Format - Unlimited budget
        let markdown_ctx = db
            .get_context_optimized(None, 10, traz_core::OutputFormat::Markdown, None, false)
            .unwrap();
        assert!(markdown_ctx.contains("# traz — Engineering Context Summary"));
        assert!(markdown_ctx.contains("Event A"));
        assert!(markdown_ctx.contains("Event B"));
        assert!(markdown_ctx.contains("Event C"));

        // 2. Dense Format - Unlimited budget
        let dense_ctx = db
            .get_context_optimized(None, 10, traz_core::OutputFormat::Dense, None, false)
            .unwrap();
        assert!(dense_ctx.contains("traz|events:3"));
        assert!(dense_ctx.contains("Event A"));

        // 3. Budget Truncation (strict low token budget)
        // With a budget of 20 tokens, only the header should fit, truncating the rest
        let truncated_ctx = db
            .get_context_optimized(None, 10, traz_core::OutputFormat::Markdown, Some(20), false)
            .unwrap();
        assert!(traz_core::estimate_tokens(&truncated_ctx) <= 35);
        assert!(truncated_ctx.contains("# traz"));
        assert!(!truncated_ctx.contains("Event A")); // Should be truncated
    }
}
