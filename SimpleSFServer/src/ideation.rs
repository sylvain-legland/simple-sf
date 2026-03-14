use axum::{
// Ref: FT-SSF-006
    extract::{State, Path, Json},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Sse},
    response::sse::Event,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use crate::{AppState, auth::extract_claims, llm};

#[derive(Deserialize)]
pub struct CreateIdeationReq {
    pub topic: String,
    pub agents: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct IdeationMsg {
    pub id: String,
    pub agent: String,
    pub content: String,
    pub turn: i32,
    pub created_at: String,
}

const DEFAULT_AGENTS: &[(&str, &str)] = &[
    ("Product Manager", "You are a pragmatic Product Manager focused on user value and business impact. Analyze the idea from a product perspective."),
    ("Tech Lead", "You are a seasoned Tech Lead focused on technical feasibility, architecture, and implementation. Analyze from a technical perspective."),
    ("UX Designer", "You are a creative UX Designer focused on user experience and design. Analyze from a design and usability perspective."),
];

pub async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, topic, status, agents, created_at FROM ideation_sessions ORDER BY created_at DESC"
        )?;
        let rows: Vec<serde_json::Value> = stmt.query_map([], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, String>(0)?,
                "topic": r.get::<_, String>(1)?,
                "status": r.get::<_, String>(2)?,
                "agents": r.get::<_, String>(3)?,
                "created_at": r.get::<_, String>(4)?,
            }))
        })?.collect::<Result<_, _>>()?;
        Ok(rows)
    });
    match r {
        Ok(sessions) => Json(serde_json::json!({"sessions": sessions})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn get_one(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let r = state.db.with(|conn| {
        let session = conn.query_row(
            "SELECT id, topic, status, agents, created_at FROM ideation_sessions WHERE id = ?1",
            [&id], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "topic": r.get::<_, String>(1)?,
                    "status": r.get::<_, String>(2)?,
                    "agents": r.get::<_, String>(3)?,
                    "created_at": r.get::<_, String>(4)?,
                }))
            },
        ).map_err(|e| anyhow::anyhow!(e))?;

        let mut stmt = conn.prepare(
            "SELECT id, agent, content, turn, created_at FROM ideation_messages WHERE session_id = ?1 ORDER BY turn, created_at"
        )?;
        let messages: Vec<IdeationMsg> = stmt.query_map([&id], |r| {
            Ok(IdeationMsg {
                id: r.get(0)?,
                agent: r.get(1)?,
                content: r.get(2)?,
                turn: r.get(3)?,
                created_at: r.get(4)?,
            })
        })?.collect::<Result<_, _>>()?;

        Ok((session, messages))
    });

    match r {
        Ok((session, messages)) => Json(serde_json::json!({
            "session": session,
            "messages": messages
        })).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response(),
    }
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateIdeationReq>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let id = Uuid::new_v4().to_string();
    let agents: Vec<String> = req.agents
        .unwrap_or_else(|| DEFAULT_AGENTS.iter().map(|(n, _)| n.to_string()).collect());
    let agents_json = serde_json::to_string(&agents).unwrap_or_default();

    let r = state.db.with(|conn| {
        conn.execute(
            "INSERT INTO ideation_sessions (id, topic, status, agents) VALUES (?1, ?2, 'pending', ?3)",
            rusqlite::params![id, req.topic, agents_json],
        )?;
        Ok(())
    });

    match r {
        Ok(_) => (StatusCode::CREATED, Json(serde_json::json!({"id": id, "topic": req.topic, "status": "pending"}))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

pub async fn start(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }
    let _ = state.db.with(|conn| {
        conn.execute("UPDATE ideation_sessions SET status='running' WHERE id=?1", [&id])?;
        Ok(())
    });

    // Spawn background ideation task
    let state_clone = Arc::clone(&state);
    let id_clone = id.clone();
    tokio::spawn(async move {
        run_ideation(state_clone, id_clone).await;
    });

    Json(serde_json::json!({"ok": true, "status": "running"})).into_response()
}

async fn run_ideation(state: Arc<AppState>, session_id: String) {
    let topic = state.db.with(|conn| {
        conn.query_row(
            "SELECT topic FROM ideation_sessions WHERE id = ?1",
            [&session_id], |r| r.get::<_, String>(0),
        ).map_err(|e| anyhow::anyhow!(e))
    }).unwrap_or_default();

    for (turn, (agent_name, agent_prompt)) in DEFAULT_AGENTS.iter().enumerate() {
        let msg = format!(
            "Topic: {}\n\nPlease provide your analysis and perspective on this idea. Be concise (2-3 paragraphs). \
             Then end with a brief 'Recommendation:' line.",
            topic
        );

        let messages = vec![llm::LLMMessage { role: "user".to_string(), content: msg }];
        let system = format!("{} The topic being discussed is: {}", agent_prompt, topic);

        match llm::chat_completion(messages, Some(&system)).await {
            Ok(content) => {
                let msg_id = Uuid::new_v4().to_string();
                let _ = state.db.with(|conn| {
                    conn.execute(
                        "INSERT INTO ideation_messages (id, session_id, agent, content, turn) VALUES (?1, ?2, ?3, ?4, ?5)",
                        rusqlite::params![msg_id, session_id, agent_name, content, turn as i32],
                    )?;
                    Ok(())
                });
            }
            Err(e) => tracing::warn!("Ideation agent {} failed: {}", agent_name, e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    let _ = state.db.with(|conn| {
        conn.execute("UPDATE ideation_sessions SET status='done' WHERE id=?1", [&session_id])?;
        Ok(())
    });
}

pub async fn stream(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if extract_claims(&headers, &state.jwt_secret).is_none() {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response();
    }

    let sse_stream = async_stream::stream! {
        let mut last_count = 0usize;
        for _ in 0..120 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let result = state.db.with(|conn| {
                let count: usize = conn.query_row(
                    "SELECT COUNT(*) FROM ideation_messages WHERE session_id = ?1",
                    [&id], |r| r.get(0),
                )?;
                let status: String = conn.query_row(
                    "SELECT status FROM ideation_sessions WHERE id = ?1",
                    [&id], |r| r.get(0),
                ).unwrap_or_else(|_| "unknown".to_string());

                if count > last_count {
                    let mut stmt = conn.prepare(
                        "SELECT id, agent, content, turn FROM ideation_messages WHERE session_id = ?1 ORDER BY turn, created_at"
                    )?;
                    let messages: Vec<serde_json::Value> = stmt.query_map([&id], |r| {
                        Ok(serde_json::json!({
                            "id": r.get::<_, String>(0)?,
                            "agent": r.get::<_, String>(1)?,
                            "content": r.get::<_, String>(2)?,
                            "turn": r.get::<_, i32>(3)?,
                        }))
                    })?.collect::<Result<_, _>>()?;
                    return Ok((count, status, Some(messages)));
                }
                Ok((count, status, None))
            });

            match result {
                Ok((count, status, Some(messages))) => {
                    last_count = count;
                    let data = serde_json::json!({"messages": messages, "status": status}).to_string();
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(data));
                    if status == "done" {
                        yield Ok(Event::default().data("[DONE]"));
                        break;
                    }
                }
                Ok((_, status, None)) => {
                    if status == "done" {
                        yield Ok(Event::default().data("[DONE]"));
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    Sse::new(sse_stream).into_response()
}
