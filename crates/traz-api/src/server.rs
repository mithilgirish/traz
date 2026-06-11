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
        .route("/events/{id}", delete(delete_event))
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
