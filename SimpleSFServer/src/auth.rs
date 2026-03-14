use axum::{
    extract::{State, Json},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::{AppState, models::Claims};
use uuid::Uuid;
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub ok: bool,
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let result = state.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, email, display_name, role, password_hash FROM users WHERE email = ?1"
        )?;
        let row = stmt.query_row([&req.email], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        });
        Ok(row)
    });

    match result {
        Ok(Ok((id, email, display_name, role, hash))) => {
            let valid = bcrypt::verify(&req.password, &hash).unwrap_or(false);

            if !valid {
                return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"ok": false, "error": "Invalid credentials"}))).into_response();
            }

            let token = make_token(&id, &email, &role, &state.jwt_secret);
            Json(AuthResponse {
                ok: true,
                token,
                user: UserInfo { id, email, display_name, role },
            }).into_response()
        }
        _ => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"ok": false, "error": "Invalid credentials"}))).into_response(),
    }
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let hash = bcrypt::hash(&req.password, 12).unwrap_or_default();
    let name = req.display_name.unwrap_or_else(|| req.email.split('@').next().unwrap_or("User").to_string());

    let r = state.db.with(|conn| {
        conn.execute(
            "INSERT INTO users (id, email, display_name, role, password_hash) VALUES (?1, ?2, ?3, 'user', ?4)",
            rusqlite::params![id, req.email, name, hash],
        )?;
        Ok(())
    });

    match r {
        Ok(_) => {
            let token = make_token(&id, &req.email, "user", &state.jwt_secret);
            Json(serde_json::json!({"ok": true, "token": token})).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"ok": false, "error": e.to_string()}))).into_response(),
    }
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match extract_claims(&headers, &state.jwt_secret) {
        Some(claims) => {
            let r = state.db.with(|conn| {
                conn.query_row(
                    "SELECT id, email, display_name, role FROM users WHERE id = ?1",
                    [&claims.sub],
                    |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
                ).map_err(|e| anyhow::anyhow!(e))
            });
            match r {
                Ok((id, email, display_name, role)) =>
                    Json(serde_json::json!({"id": id, "email": email, "display_name": display_name, "role": role})).into_response(),
                _ => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response(),
            }
        }
        None => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "unauthorized"}))).into_response(),
    }
}

pub fn make_token(user_id: &str, email: &str, role: &str, secret: &str) -> String {
    let exp = chrono::Utc::now().timestamp() + 86400 * 30;
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        exp,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .unwrap_or_default()
}

pub fn extract_claims(headers: &HeaderMap, secret: &str) -> Option<Claims> {
    let auth = headers.get("Authorization")?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ).ok()?;
    Some(data.claims)
}
