use axum::{
// Ref: FT-SSF-003
    extract::{State, Path, Json},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::{AppState, auth::extract_claims};

#[derive(Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tech_stack: String,
    pub status: String,
    pub progress: f64,
    pub current_phase: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct CreateProject {
    pub name: String,
    pub description: Option<String>,
    pub tech_stack: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tech_stack: Option<String>,
}

fn require_auth(headers: &HeaderMap, state: &AppState) -> Option<String> {
    extract_claims(headers, &state.jwt_secret).map(|c| c.sub)
}

fn row_to_project(r: &rusqlite::Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: r.get(0)?,
        name: r.get(1)?,
        description: r.get(2)?,
        tech_stack: r.get(3)?,
        status: r.get(4)?,
        progress: r.get(5)?,
        current_phase: r.get(6)?,
        created_at: r.get(7)?,
        updated_at: r.get(8)?,
    })
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if require_auth(&headers, &state).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, tech_stack, status, progress, current_phase, created_at, updated_at FROM projects ORDER BY created_at DESC"
        )?;
        let rows = stmt.query_map([], row_to_project)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    });
    match r {
        Ok(projects) => Json(serde_json::json!({"projects": projects})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if require_auth(&headers, &state).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        conn.query_row(
            "SELECT id, name, description, tech_stack, status, progress, current_phase, created_at, updated_at FROM projects WHERE id = ?1",
            [&id], row_to_project,
        ).map_err(|e| anyhow::anyhow!(e))
    });
    match r {
        Ok(p) => Json(p).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response(),
    }
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateProject>,
) -> impl IntoResponse {
    if require_auth(&headers, &state).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let id = Uuid::new_v4().to_string();
    let r = state.db.with(|conn| {
        conn.execute(
            "INSERT INTO projects (id, name, description, tech_stack) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, req.name, req.description.unwrap_or_default(), req.tech_stack.unwrap_or_default()],
        )?;
        conn.query_row(
            "SELECT id, name, description, tech_stack, status, progress, current_phase, created_at, updated_at FROM projects WHERE id = ?1",
            [&id], row_to_project,
        ).map_err(|e| anyhow::anyhow!(e))
    });
    match r {
        Ok(p) => (StatusCode::CREATED, Json(p)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateProject>,
) -> impl IntoResponse {
    if require_auth(&headers, &state).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        if let Some(n) = &req.name { conn.execute("UPDATE projects SET name=?1, updated_at=datetime('now') WHERE id=?2", rusqlite::params![n, id])?; }
        if let Some(d) = &req.description { conn.execute("UPDATE projects SET description=?1, updated_at=datetime('now') WHERE id=?2", rusqlite::params![d, id])?; }
        if let Some(t) = &req.tech_stack { conn.execute("UPDATE projects SET tech_stack=?1, updated_at=datetime('now') WHERE id=?2", rusqlite::params![t, id])?; }
        conn.query_row(
            "SELECT id, name, description, tech_stack, status, progress, current_phase, created_at, updated_at FROM projects WHERE id = ?1",
            [&id], row_to_project,
        ).map_err(|e| anyhow::anyhow!(e))
    });
    match r {
        Ok(p) => Json(p).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn remove(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if require_auth(&headers, &state).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        conn.execute("DELETE FROM projects WHERE id = ?1", [&id])?;
        Ok(())
    });
    match r {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn start(State(state): State<Arc<AppState>>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    set_status(&state, &headers, &id, "running", Some("Analyzing requirements"), Some(0.05)).await
}

pub async fn stop(State(state): State<Arc<AppState>>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    set_status(&state, &headers, &id, "idle", Some(""), Some(0.0)).await
}

pub async fn pause(State(state): State<Arc<AppState>>, headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    set_status(&state, &headers, &id, "paused", None, None).await
}

async fn set_status(state: &AppState, headers: &HeaderMap, id: &str, status: &str, phase: Option<&str>, progress: Option<f64>) -> impl IntoResponse {
    if extract_claims(headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        if let (Some(ph), Some(pr)) = (phase, progress) {
            conn.execute(
                "UPDATE projects SET status=?1, current_phase=?2, progress=?3, updated_at=datetime('now') WHERE id=?4",
                rusqlite::params![status, ph, pr, id],
            )?;
        } else {
            conn.execute(
                "UPDATE projects SET status=?1, updated_at=datetime('now') WHERE id=?2",
                rusqlite::params![status, id],
            )?;
        }
        Ok(())
    });
    match r {
        Ok(_) => Json(serde_json::json!({"ok": true, "status": status})).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
