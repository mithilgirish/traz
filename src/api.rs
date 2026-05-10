use crate::db::Db;
use crate::models::Event;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

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

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/events", post(create_event).get(list_events))
        .route("/events/{id}", delete(delete_event))
        .route("/health", get(health))
        .route("/stats", get(stats))
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

    Json(serde_json::json!({
        "total_events": count,
        "by_tool": tools,
        "db_path": state.db.path().to_string_lossy(),
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
                StatusCode::INTERNAL_SERVER_ERROR,
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
    let limit = filter.limit.unwrap_or(50);

    let result = if let Some(ref search) = filter.search {
        state.db.search_events(search)
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
                Json(serde_json::json!({ "error": e.to_string() })),
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
                Json(serde_json::json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    }
}
