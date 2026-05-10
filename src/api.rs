use crate::db::Db;
use crate::models::Event;
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

/// Maximum request body size: 64 KB.
/// Prevents memory exhaustion from oversized payloads.
const MAX_BODY_SIZE: usize = 64 * 1024;

// ── State ───────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
}

// ── Request / query types ───────────────────────────────────────────

#[derive(Deserialize)]
pub struct EventPayload {
    pub tool: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub title: String,
    pub summary: Option<String>,
    pub files: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct FilterQuery {
    pub limit: Option<u32>,
    pub tool: Option<String>,
    pub event_type: Option<String>,
    pub search: Option<String>,
}

// ── Router ──────────────────────────────────────────────────────────

pub fn create_router(db: Arc<Db>) -> Router {
    let state = AppState { db };

    // CORS: only allow requests from localhost origins.
    // This prevents random websites from calling the traz API
    // while still allowing local tools and scripts.
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
        .route("/health", get(health))
        .route("/stats", get(stats))
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

// ── Handlers ────────────────────────────────────────────────────────

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    let count = state.db.count_events().unwrap_or(0);
    let by_tool = state.db.get_stats().unwrap_or_default();

    let tools: serde_json::Value = by_tool
        .into_iter()
        .map(|(tool, cnt)| serde_json::json!({ "tool": tool, "count": cnt }))
        .collect();

    // Don't expose full filesystem path — just show the filename
    let db_name = state
        .db
        .path()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "traz.db".to_string());

    Json(serde_json::json!({
        "total_events": count,
        "by_tool": tools,
        "db": db_name,
    }))
}

async fn create_event(
    State(state): State<AppState>,
    Json(payload): Json<EventPayload>,
) -> impl IntoResponse {
    let event = Event::new(
        payload.tool,
        payload.event_type,
        payload.title,
        payload.summary,
        payload.files,
        None,
    );

    match state.db.insert_event(&event) {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "id": id, "status": "created" })),
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

    let result = if let Some(ref search) = filter.search {
        state.db.search_events(search, limit)
    } else {
        state
            .db
            .get_filtered_events(limit, filter.tool, filter.event_type)
    };

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

async fn delete_event(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
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
