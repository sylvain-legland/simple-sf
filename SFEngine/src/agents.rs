use crate::db;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub persona: String,
    pub system_prompt: String,
    pub model: String,
    pub avatar: String,
    pub color: String,
    pub tagline: String,
    pub hierarchy_rank: i64,
}

fn agent_from_row(row: &rusqlite::Row) -> rusqlite::Result<Agent> {
    Ok(Agent {
        id: row.get(0)?,
        name: row.get(1)?,
        role: row.get(2)?,
        persona: row.get(3)?,
        system_prompt: row.get(4)?,
        model: row.get(5)?,
        avatar: row.get(6)?,
        color: row.get(7)?,
        tagline: row.get(8)?,
        hierarchy_rank: row.get(9)?,
    })
}

const SELECT_COLS: &str = "id, name, role, persona, system_prompt, model, avatar, color, tagline, hierarchy_rank";

pub fn get_agent(id: &str) -> Option<Agent> {
    db::with_db(|conn| {
        let sql = format!("SELECT {} FROM agents WHERE id = ?1", SELECT_COLS);
        conn.query_row(&sql, [id], agent_from_row).ok()
    })
}

pub fn get_agents_for_roles(roles: &[&str]) -> Vec<Agent> {
    db::with_db(|conn| {
        let placeholders: Vec<String> = roles.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!("SELECT {} FROM agents WHERE role IN ({})", SELECT_COLS, placeholders.join(","));
        let mut stmt = conn.prepare(&sql).unwrap();
        let params: Vec<&dyn rusqlite::types::ToSql> = roles.iter().map(|r| r as &dyn rusqlite::types::ToSql).collect();
        stmt.query_map(params.as_slice(), agent_from_row).unwrap().filter_map(|r| r.ok()).collect()
    })
}

pub fn all_agents() -> Vec<Agent> {
    db::with_db(|conn| {
        let sql = format!("SELECT {} FROM agents ORDER BY name", SELECT_COLS);
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], agent_from_row).unwrap().filter_map(|r| r.ok()).collect()
    })
}
