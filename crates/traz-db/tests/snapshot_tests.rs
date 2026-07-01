use std::path::PathBuf;
use traz_core::Event;
use traz_db::Db;

#[tokio::test]
#[ignore = "Requires downloading embedding model"]
async fn test_semantic_search_snapshot() {
    let mut db = Db::open(&PathBuf::from(":memory:")).await.unwrap();

    // Enable embeddings in the Db config directly so insert_event saves them
    db.config.embeddings_enabled = true;

    let e1 = Event::new(
        "tool1".into(),
        "feature".into(),
        "Authentication logic".into(),
        Some("Implemented login flow".into()),
        None,
        None,
    );
    db.insert_event(&e1).await.unwrap();

    let results = db
        .semantic_search("login", 1)
        .await
        .expect("Semantic search failed");
    assert!(
        !results.is_empty(),
        "Semantic search returned empty results"
    );

    let snapshot_data = results
        .iter()
        .map(|(event, _)| format!("Title: {}", event.title))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        snapshot_data.contains("Authentication logic"),
        "Snapshot did not contain expected data. Got: {}",
        snapshot_data
    );
}
