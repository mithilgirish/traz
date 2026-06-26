use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use traz_core::Event;
use traz_db::Db;

/// Maximum request body size: 10 MB.
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

#[derive(Clone)]
struct AppState {
    db: Arc<Db>,
}

#[derive(Deserialize)]
struct EventPayload {
    tool: String,
    #[serde(rename = "type")]
    event_type: String,
    title: String,
    summary: Option<String>,
    files: Option<Vec<String>>,
    metadata: Option<serde_json::Value>,
    tags: Option<Vec<String>>,
    session_id: Option<String>,
    diff: Option<String>,
}

#[derive(Deserialize)]
struct FilterQuery {
    limit: Option<u32>,
    tool: Option<String>,
    event_type: Option<String>,
    search: Option<String>,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
}

async fn validate_host(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    if let Some(host) = req.headers().get(axum::http::header::HOST) {
        let host_str = host.to_str().unwrap_or_default();
        if host_str == "localhost"
            || host_str.starts_with("127.0.0.1")
            || host_str.starts_with("localhost:")
            || host_str.starts_with("[::1]")
        {
            return Ok(next.run(req).await);
        }
    }
    Err(axum::http::StatusCode::BAD_REQUEST)
}

/// Build the traz REST API router.
pub fn create_router(db: Arc<Db>) -> Router {
    let state = AppState { db };

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            let origin = origin.as_bytes();
            origin.starts_with(b"http://localhost")
                || origin.starts_with(b"http://127.0.0.1")
                || origin.starts_with(b"http://[::1]")
        }))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/events", post(create_event).get(list_events))
        .route("/events/:id", delete(delete_event))
        .route("/search", get(search_events))
        .route("/timeline", get(timeline))
        .route("/context", get(context_summary))
        .route("/health", get(health))
        .route("/stats", get(stats))
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .route_layer(axum::middleware::from_fn(validate_host))
        .with_state(state)
}

// ── Handlers ────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "service": "traz"
    }))
}

async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    let db_clone = state.db.clone();
    let (count, by_tool) = tokio::task::spawn_blocking(move || {
        let count = db_clone.count_events().unwrap_or(0);
        let by_tool = db_clone.get_stats().unwrap_or_default();
        (count, by_tool)
    })
    .await
    .unwrap_or_default();

    let tools: serde_json::Value = by_tool
        .into_iter()
        .map(|(tool, cnt)| serde_json::json!({ "tool": tool, "count": cnt }))
        .collect();

    Json(serde_json::json!({
        "total_events": count,
        "by_tool": tools,
    }))
}

async fn create_event(
    State(state): State<AppState>,
    Json(payload): Json<EventPayload>,
) -> impl IntoResponse {
    let mut event = Event::new(
        payload.tool,
        payload.event_type,
        payload.title,
        payload.summary,
        payload.files,
        None,
    );

    if let Some(metadata) = payload.metadata {
        event = event.with_metadata(metadata);
    }
    if let Some(tags) = payload.tags {
        event = event.with_tags(tags);
    }
    if let Some(session_id) = payload.session_id {
        event = event.with_session(session_id);
    }
    if let Some(diff) = payload.diff {
        event = event.with_diff(diff);
    }

    let db_clone = state.db.clone();
    let event_uuid = event.uuid.clone();
    let result = tokio::task::spawn_blocking(move || db_clone.insert_event(&event))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e)));

    match result {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "id": id, "uuid": event_uuid, "status": "created" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to insert event: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}

async fn list_events(
    State(state): State<AppState>,
    Query(filter): Query<FilterQuery>,
) -> impl IntoResponse {
    let limit = filter.limit.unwrap_or(50).min(500);

    let db_clone = state.db.clone();
    let result = tokio::task::spawn_blocking(move || {
        db_clone.get_filtered_events(
            limit,
            filter.tool,
            filter.event_type,
            filter.since,
            filter.until,
        )
    })
    .await
    .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e)));

    match result {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(e) => {
            tracing::error!("Failed to fetch events: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to query events" })),
            )
                .into_response()
        }
    }
}

async fn search_events(
    State(state): State<AppState>,
    Query(filter): Query<FilterQuery>,
) -> impl IntoResponse {
    let query = filter.search.unwrap_or_default();
    if query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Missing 'search' query parameter" })),
        )
            .into_response();
    }

    let limit = filter.limit.unwrap_or(50).min(500);
    let db_clone = state.db.clone();
    let tool_filter = filter.tool;
    let event_type_filter = filter.event_type;

    let result = tokio::task::spawn_blocking(move || {
        let filters = traz_db::SearchFilters {
            tool: tool_filter.as_deref(),
            event_type: event_type_filter.as_deref(),
            ..Default::default()
        };
        db_clone
            .hybrid_search(&query, &filters, limit)
            .map(|events_with_scores| {
                events_with_scores
                    .into_iter()
                    .map(|(event, _)| event)
                    .collect::<Vec<Event>>()
            })
    })
    .await
    .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e)));

    match result {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(e) => {
            tracing::error!("Search failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Search failed" })),
            )
                .into_response()
        }
    }
}

async fn timeline(
    State(state): State<AppState>,
    Query(filter): Query<FilterQuery>,
) -> impl IntoResponse {
    let limit = filter.limit.unwrap_or(200).min(500);
    let db_clone = state.db.clone();
    let result = tokio::task::spawn_blocking(move || db_clone.get_timeline(limit))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e)));

    match result {
        Ok(events) => (StatusCode::OK, Json(events)).into_response(),
        Err(e) => {
            tracing::error!("Timeline query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Timeline query failed" })),
            )
                .into_response()
        }
    }
}

async fn delete_event(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let db_clone = state.db.clone();
    let result = tokio::task::spawn_blocking(move || db_clone.delete_event(id))
        .await
        .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e)));

    match result {
        Ok(true) => (
            StatusCode::OK,
            Json(serde_json::json!({ "id": id, "status": "deleted" })),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Event {} not found", id) })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to delete event {}: {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to delete event" })),
            )
                .into_response()
        }
    }
}

async fn context_summary(
    State(state): State<AppState>,
    Query(filter): Query<FilterQuery>,
) -> impl IntoResponse {
    let limit = filter.limit.unwrap_or(10).min(100);
    let search = filter.search.clone();
    let db_clone = state.db.clone();
    let result =
        tokio::task::spawn_blocking(move || db_clone.get_context_summary(search.as_deref(), limit))
            .await
            .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e)));

    match result {
        Ok(ctx) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "context": ctx,
                "format": "markdown",
                "version": env!("CARGO_PKG_VERSION")
            })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Context summary failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to generate context summary" })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use std::time::SystemTime;
    use tower::ServiceExt;

    fn setup_test_env(test_name: &str) -> (Arc<Db>, std::path::PathBuf) {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let unique_dir = std::env::temp_dir().join(format!("traz_api_test_{}_{}", test_name, ts));
        let _ = std::fs::create_dir_all(&unique_dir);
        let db_path = unique_dir.join("traz.db");
        let db = Db::open(&db_path).unwrap();
        (Arc::new(db), unique_dir)
    }

    fn cleanup_test_env(unique_dir: std::path::PathBuf) {
        let _ = std::fs::remove_dir_all(unique_dir);
    }

    #[tokio::test]
    async fn test_api_health() {
        let (db, test_dir) = setup_test_env("health");
        let app = create_router(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body_json["status"], "ok");
        assert_eq!(body_json["service"], "traz");

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_api_create_and_list_events() {
        let (db, test_dir) = setup_test_env("events");
        let app = create_router(db);

        // 1. Create an event
        let payload = serde_json::json!({
            "tool": "cursor",
            "type": "feature",
            "title": "Add traz-api unit tests",
            "summary": "Implemented route testing using tower oneshot",
            "files": ["crates/traz-api/src/server.rs"],
            "tags": ["testing", "api"]
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events")
                    .header(header::HOST, "localhost:3000")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body_json["status"], "created");
        assert!(body_json.get("id").is_some());

        // 2. List the events
        let response_list = app
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response_list.status(), StatusCode::OK);
        let body_list = axum::body::to_bytes(response_list.into_body(), 10240)
            .await
            .unwrap();
        let list_json: serde_json::Value = serde_json::from_slice(&body_list).unwrap();

        let events_arr = list_json.as_array().unwrap();
        assert_eq!(events_arr.len(), 1);
        assert_eq!(events_arr[0]["tool"], "cursor");
        assert_eq!(events_arr[0]["type"], "feature");
        assert_eq!(events_arr[0]["title"], "Add traz-api unit tests");

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_api_host_validation() {
        let (db, test_dir) = setup_test_env("host_val");
        let app = create_router(db);

        // Host validation should block external domains like google.com
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .header(header::HOST, "google.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_api_stats() {
        let (db, test_dir) = setup_test_env("stats");
        let app = create_router(db.clone());

        // Insert mock event
        db.insert_event(&traz_core::Event::new(
            "aider".to_string(),
            "bug_fix".to_string(),
            "Title".to_string(),
            None,
            None,
            None,
        ))
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/stats")
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body_json["total_events"], 1);
        let by_tool = body_json["by_tool"].as_array().unwrap();
        assert_eq!(by_tool[0]["tool"], "aider");
        assert_eq!(by_tool[0]["count"], 1);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_api_delete_event() {
        let (db, test_dir) = setup_test_env("delete");
        let app = create_router(db.clone());

        let id = db
            .insert_event(&traz_core::Event::new(
                "cursor".to_string(),
                "refactor".to_string(),
                "Title".to_string(),
                None,
                None,
                None,
            ))
            .unwrap();

        // 1. Delete existing
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/events/{}", id))
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body_json["status"], "deleted");
        assert_eq!(body_json["id"], id);

        // 2. Delete non-existent
        let response_missing = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/events/{}", id))
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response_missing.status(), StatusCode::NOT_FOUND);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_api_search_events() {
        let (db, test_dir) = setup_test_env("search");
        let app = create_router(db.clone());

        db.insert_event(&traz_core::Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "Target search string".to_string(),
            None,
            None,
            None,
        ))
        .unwrap();

        // 1. Valid search
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/search?search=Target")
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 10240)
            .await
            .unwrap();
        let results: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["title"], "Target search string");

        // 2. Missing search query param -> 400 Bad Request
        let response_bad = app
            .oneshot(
                Request::builder()
                    .uri("/search")
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response_bad.status(), StatusCode::BAD_REQUEST);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_api_create_invalid_payload() {
        let (db, test_dir) = setup_test_env("invalid_payload");
        let app = create_router(db);

        // Missing required fields (e.g. type/event_type, title, tool)
        let payload = serde_json::json!({
            "summary": "This payload misses 'tool', 'type', and 'title'"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/events")
                    .header(header::HOST, "localhost:3000")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum's Json extractor returns 422 Unprocessable Entity for invalid structure
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        cleanup_test_env(test_dir);
    }

    #[tokio::test]
    async fn test_api_context_summary() {
        let (db, test_dir) = setup_test_env("context");
        let app = create_router(db.clone());

        db.insert_event(&traz_core::Event::new(
            "cursor".to_string(),
            "feature".to_string(),
            "Testing context endpoint".to_string(),
            None,
            None,
            None,
        ))
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/context?limit=5")
                    .header(header::HOST, "localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 10240)
            .await
            .unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert!(
            body_json["context"]
                .as_str()
                .unwrap()
                .contains("Testing context endpoint")
        );
        assert_eq!(body_json["format"], "markdown");

        cleanup_test_env(test_dir);
    }
}
