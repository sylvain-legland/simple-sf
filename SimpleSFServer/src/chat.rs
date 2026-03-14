use axum::{
// Ref: FT-SSF-001
    extract::{State, Path, Json},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Sse},
    response::sse::Event,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use futures_util::stream;
use crate::{AppState, auth::extract_claims, llm};

#[derive(Deserialize)]
pub struct CreateSessionReq {
    pub project_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Deserialize)]
pub struct MessageReq {
    pub content: String,
    pub system: Option<String>,
}

#[derive(Serialize)]
pub struct MessageResp {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub async fn create_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateSessionReq>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let id = Uuid::new_v4().to_string();
    let title = req.title.unwrap_or_else(|| "Jarvis Chat".to_string());
    let r = state.db.with(|conn| {
        conn.execute(
            "INSERT INTO chat_sessions (id, project_id, title) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, req.project_id, title],
        )?;
        Ok(())
    });
    match r {
        Ok(_) => Json(serde_json::json!({"id": id, "title": title})).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(req): Json<MessageReq>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    // Save user message
    let user_msg_id = Uuid::new_v4().to_string();
    let _ = state.db.with(|conn| {
        conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content) VALUES (?1, ?2, 'user', ?3)",
            rusqlite::params![user_msg_id, session_id, req.content],
        )?;
        Ok(())
    });

    // Get history
    let history = state.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT role, content FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC LIMIT 20"
        )?;
        let rows = stmt.query_map([&session_id], |r| {
            Ok(llm::LLMMessage { role: r.get(0)?, content: r.get(1)? })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }).unwrap_or_default();

    let system = req.system.as_deref().unwrap_or(
        "You are Jarvis, an expert software engineering assistant embedded in Simple SF. \
         Help users design, plan, and build software projects. Be concise and practical."
    );

    let response = llm::chat_completion(history, Some(system)).await;
    match response {
        Ok(content) => {
            let resp_id = Uuid::new_v4().to_string();
            let _ = state.db.with(|conn| {
                conn.execute(
                    "INSERT INTO chat_messages (id, session_id, role, content) VALUES (?1, ?2, 'assistant', ?3)",
                    rusqlite::params![resp_id, session_id, content],
                )?;
                Ok(())
            });
            Json(serde_json::json!({"id": resp_id, "role": "assistant", "content": content})).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn stream_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(req): Json<MessageReq>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let user_msg_id = Uuid::new_v4().to_string();
    let _ = state.db.with(|conn| {
        conn.execute(
            "INSERT INTO chat_messages (id, session_id, role, content) VALUES (?1, ?2, 'user', ?3)",
            rusqlite::params![user_msg_id, session_id, req.content],
        )?;
        Ok(())
    });

    let history = state.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT role, content FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC LIMIT 20"
        )?;
        let rows = stmt.query_map([&session_id], |r| {
            Ok(llm::LLMMessage { role: r.get(0)?, content: r.get(1)? })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }).unwrap_or_default();

    let system = req.system.unwrap_or_else(|| {
        "You are Jarvis, an expert software engineering assistant. Be concise and practical.".to_string()
    });

    // For streaming, we do a regular call then stream it word by word
    let state_clone = Arc::clone(&state);
    let session_id_clone = session_id.clone();

    let sse_stream = async_stream::stream! {
        match llm::chat_completion(history, Some(system.as_str())).await {
            Ok(content) => {
                // Save full response
                let resp_id = Uuid::new_v4().to_string();
                let _ = state_clone.db.with(|conn| {
                    conn.execute(
                        "INSERT INTO chat_messages (id, session_id, role, content) VALUES (?1, ?2, 'assistant', ?3)",
                        rusqlite::params![resp_id, session_id_clone, content],
                    )?;
                    Ok(())
                });
                // Stream word by word
                for word in content.split_inclusive(' ') {
                    let data = serde_json::json!({"token": word}).to_string();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(data));
                    tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                }
                yield Ok(Event::default().data("[DONE]"));
            }
            Err(e) => {
                let data = serde_json::json!({"error": e.to_string()}).to_string();
                yield Ok(Event::default().data(data));
            }
        }
    };

    Sse::new(sse_stream).into_response()
}

pub async fn get_history(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, role, content, created_at FROM chat_messages WHERE session_id = ?1 ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map([&session_id], |r| {
            Ok(MessageResp {
                id: r.get(0)?,
                role: r.get(1)?,
                content: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    });
    match r {
        Ok(msgs) => Json(serde_json::json!({"messages": msgs})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
