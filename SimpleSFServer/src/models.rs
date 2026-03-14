use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
// Ref: FT-SSF-023
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tech_stack: String,
    pub status: String,       // idle | running | paused | done | failed
    pub progress: f32,        // 0.0 - 1.0
    pub current_phase: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,   // user | assistant | system
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeationSession {
    pub id: String,
    pub topic: String,
    pub status: String,  // pending | running | done
    pub agents: Vec<String>,
    pub messages: Vec<IdeationMessage>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeationMessage {
    pub id: String,
    pub session_id: String,
    pub agent: String,
    pub content: String,
    pub turn: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // user id
    pub email: String,
    pub role: String,
    pub exp: i64,
}
