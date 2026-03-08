use crate::db;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub persona: String,
    pub model: String,
}

pub fn get_agent(id: &str) -> Option<Agent> {
    db::with_db(|conn| {
        conn.query_row(
            "SELECT id, name, role, persona, model FROM agents WHERE id = ?1",
            [id],
            |row| Ok(Agent {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                persona: row.get(3)?,
                model: row.get(4)?,
            }),
        ).ok()
    })
}

pub fn get_agents_for_roles(roles: &[&str]) -> Vec<Agent> {
    db::with_db(|conn| {
        let placeholders: Vec<String> = roles.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
        let sql = format!("SELECT id, name, role, persona, model FROM agents WHERE role IN ({})", placeholders.join(","));
        let mut stmt = conn.prepare(&sql).unwrap();
        let params: Vec<&dyn rusqlite::types::ToSql> = roles.iter().map(|r| r as &dyn rusqlite::types::ToSql).collect();
        stmt.query_map(params.as_slice(), |row| Ok(Agent {
            id: row.get(0)?,
            name: row.get(1)?,
            role: row.get(2)?,
            persona: row.get(3)?,
            model: row.get(4)?,
        })).unwrap().filter_map(|r| r.ok()).collect()
    })
}

pub fn all_agents() -> Vec<Agent> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare("SELECT id, name, role, persona, model FROM agents").unwrap();
        stmt.query_map([], |row| Ok(Agent {
            id: row.get(0)?,
            name: row.get(1)?,
            role: row.get(2)?,
            persona: row.get(3)?,
            model: row.get(4)?,
        })).unwrap().filter_map(|r| r.ok()).collect()
    })
}
