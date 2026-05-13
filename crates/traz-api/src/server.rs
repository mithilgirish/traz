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

/// Maximum request body size: 64 KB.
const MAX_BODY_SIZE: usize = 64 * 1024;

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
        .route("/events/{id}", delete(delete_event))
        .route("/search", get(search_events))
        .route("/timeline", get(timeline))
        .route("/health", get(health))
        .route("/stats", get(stats))
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
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
    let count = state.db.count_events().unwrap_or(0);
    let by_tool = state.db.get_stats().unwrap_or_default();

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

    match state.db.insert_event(&event) {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "id": id, "uuid": event.uuid, "status": "created" })),
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

    let result = state.db.get_filtered_events(
        limit,
        filter.tool,
        filter.event_type,
        filter.since,
        filter.until,
    );

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
    match state.db.search_events(&query, filter.tool.as_deref(), limit) {
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
    match state.db.get_timeline(limit) {
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
    match state.db.delete_event(id) {
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
