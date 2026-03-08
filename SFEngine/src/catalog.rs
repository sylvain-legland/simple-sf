//! SF Platform Catalog — loads 192 agents, 1286 skills, 19 patterns, 42 workflows
//! from bundled JSON files exported from the SF platform database.
//!
//! At sf_init(), seed_from_json() reads the JSON files and inserts everything
//! into the local SQLite database. Subsequent lookups go through DB or in-memory cache.

use crate::db;
use rusqlite::params;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

// ──────────────────────────────────────────
// In-memory agent cache (for fast lookup without DB)
// ──────────────────────────────────────────

static AGENT_CACHE: OnceLock<HashMap<String, AgentInfo>> = OnceLock::new();

/// Lightweight agent info for runtime lookups
#[derive(Clone, Debug)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub persona: String,
    pub system_prompt: String,
    pub avatar: String,
    pub color: String,
    pub tagline: String,
    pub tools: Vec<String>,
    pub skills: Vec<String>,
    pub hierarchy_rank: i64,
    pub can_veto: bool,
}

/// Get agent info from cache (fast, no DB)
pub fn get_agent_info(id: &str) -> Option<AgentInfo> {
    AGENT_CACHE.get().and_then(|c| c.get(id).cloned())
}

/// Get all agent infos
pub fn all_agents() -> Vec<AgentInfo> {
    AGENT_CACHE.get()
        .map(|c| c.values().cloned().collect())
        .unwrap_or_default()
}

/// Count of loaded agents
pub fn agent_count() -> usize {
    AGENT_CACHE.get().map(|c| c.len()).unwrap_or(0)
}

// ──────────────────────────────────────────
// JSON Loading — reads from bundled SFData/
// ──────────────────────────────────────────

/// Seed all SF platform data from JSON files.
pub fn seed_from_json(data_dir: &str) {
    let mut cache = HashMap::new();

    seed_agents_json(data_dir, &mut cache);
    seed_skills_json(data_dir);
    seed_patterns_json(data_dir);
    seed_workflows_json(data_dir);

    AGENT_CACHE.set(cache).ok();
}

fn seed_agents_json(data_dir: &str, cache: &mut HashMap<String, AgentInfo>) {
    let path = format!("{}/agents.json", data_dir);
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("[catalog] No agents.json at {}, using fallback", path);
            seed_fallback_agents(cache);
            return;
        }
    };
    let agents: Vec<Value> = match serde_json::from_str(&data) {
        Ok(a) => a,
        Err(e) => { eprintln!("[catalog] Failed to parse agents.json: {}", e); return; }
    };
    let count = agents.len();

    db::with_db(|conn| {
        let mut stmt = conn.prepare_cached(
            "INSERT OR REPLACE INTO agents (id, name, role, description, system_prompt, \
             provider, model, temperature, max_tokens, skills_json, tools_json, \
             mcps_json, permissions_json, tags_json, icon, color, is_builtin, \
             avatar, tagline, persona, motivation, hierarchy_rank, project_id) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23)"
        ).unwrap();

        for a in &agents {
            let id = a["id"].as_str().unwrap_or("");
            let name = a["name"].as_str().unwrap_or("");
            let role = a["role"].as_str().unwrap_or("worker");
            let persona = a["persona"].as_str().unwrap_or("");
            let system_prompt = a["system_prompt"].as_str().unwrap_or("");
            let avatar = a["avatar"].as_str().unwrap_or("");
            let color = a["color"].as_str().unwrap_or("#f78166");
            let tagline = a["tagline"].as_str().unwrap_or("");
            let tools_json = a["tools_json"].as_str().unwrap_or("[]");
            let skills_json = a["skills_json"].as_str().unwrap_or("[]");
            let hierarchy_rank = a["hierarchy_rank"].as_i64().unwrap_or(50);
            let permissions = a["permissions_json"].as_str().unwrap_or("{}");
            let can_veto = serde_json::from_str::<Value>(permissions)
                .ok()
                .and_then(|p| p.get("can_veto")?.as_bool())
                .unwrap_or(false);

            stmt.execute(params![
                id, name, role,
                a["description"].as_str().unwrap_or(""),
                system_prompt,
                a["provider"].as_str().unwrap_or("local"),
                a["model"].as_str().unwrap_or("default"),
                a["temperature"].as_f64().unwrap_or(0.7),
                a["max_tokens"].as_i64().unwrap_or(4096),
                skills_json, tools_json,
                a["mcps_json"].as_str().unwrap_or("[]"),
                permissions,
                a["tags_json"].as_str().unwrap_or("[]"),
                a["icon"].as_str().unwrap_or("bot"),
                color,
                a["is_builtin"].as_i64().unwrap_or(0),
                avatar, tagline, persona,
                a["motivation"].as_str().unwrap_or(""),
                hierarchy_rank,
                a["project_id"].as_str().unwrap_or(""),
            ]).ok();

            let tools: Vec<String> = serde_json::from_str(tools_json).unwrap_or_default();
            let skills: Vec<String> = serde_json::from_str(skills_json).unwrap_or_default();
            cache.insert(id.to_string(), AgentInfo {
                id: id.to_string(),
                name: name.to_string(),
                role: role.to_string(),
                persona: persona.to_string(),
                system_prompt: system_prompt.to_string(),
                avatar: avatar.to_string(),
                color: color.to_string(),
                tagline: tagline.to_string(),
                tools, skills, hierarchy_rank, can_veto,
            });
        }
    });
    eprintln!("[catalog] Loaded {} agents", count);
}

fn seed_skills_json(data_dir: &str) {
    let path = format!("{}/skills.json", data_dir);
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => { eprintln!("[catalog] No skills.json found"); return; }
    };
    let skills: Vec<Value> = match serde_json::from_str(&data) {
        Ok(s) => s,
        Err(e) => { eprintln!("[catalog] Failed to parse skills.json: {}", e); return; }
    };
    let count = skills.len();
    db::with_db(|conn| {
        let mut stmt = conn.prepare_cached(
            "INSERT OR REPLACE INTO skills (id, name, description, content, source, source_url, tags_json) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)"
        ).unwrap();
        for s in &skills {
            stmt.execute(params![
                s["id"].as_str().unwrap_or(""),
                s["name"].as_str().unwrap_or(""),
                s["description"].as_str().unwrap_or(""),
                s["content"].as_str().unwrap_or(""),
                s["source"].as_str().unwrap_or(""),
                s["source_url"].as_str().unwrap_or(""),
                s["tags_json"].as_str().unwrap_or("[]"),
            ]).ok();
        }
    });
    eprintln!("[catalog] Loaded {} skills", count);
}

fn seed_patterns_json(data_dir: &str) {
    let path = format!("{}/patterns.json", data_dir);
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => { eprintln!("[catalog] No patterns.json found"); return; }
    };
    let patterns: Vec<Value> = match serde_json::from_str(&data) {
        Ok(p) => p,
        Err(e) => { eprintln!("[catalog] Failed to parse patterns.json: {}", e); return; }
    };
    let count = patterns.len();
    db::with_db(|conn| {
        let mut stmt = conn.prepare_cached(
            "INSERT OR REPLACE INTO patterns (id, name, description, type, agents_json, \
             edges_json, config_json, memory_config_json, icon, is_builtin) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"
        ).unwrap();
        for p in &patterns {
            stmt.execute(params![
                p["id"].as_str().unwrap_or(""),
                p["name"].as_str().unwrap_or(""),
                p["description"].as_str().unwrap_or(""),
                p["type"].as_str().unwrap_or("sequential"),
                p["agents_json"].as_str().unwrap_or("[]"),
                p["edges_json"].as_str().unwrap_or("[]"),
                p["config_json"].as_str().unwrap_or("{}"),
                p["memory_config_json"].as_str().unwrap_or("{}"),
                p["icon"].as_str().unwrap_or(""),
                p["is_builtin"].as_i64().unwrap_or(0),
            ]).ok();
        }
    });
    eprintln!("[catalog] Loaded {} patterns", count);
}

fn seed_workflows_json(data_dir: &str) {
    let path = format!("{}/workflows.json", data_dir);
    let data = match std::fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => { eprintln!("[catalog] No workflows.json found"); return; }
    };
    let workflows: Vec<Value> = match serde_json::from_str(&data) {
        Ok(w) => w,
        Err(e) => { eprintln!("[catalog] Failed to parse workflows.json: {}", e); return; }
    };
    let count = workflows.len();
    db::with_db(|conn| {
        let mut stmt = conn.prepare_cached(
            "INSERT OR REPLACE INTO workflows (id, name, description, phases_json, config_json, icon, is_builtin) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)"
        ).unwrap();
        for w in &workflows {
            stmt.execute(params![
                w["id"].as_str().unwrap_or(""),
                w["name"].as_str().unwrap_or(""),
                w["description"].as_str().unwrap_or(""),
                w["phases_json"].as_str().unwrap_or("[]"),
                w["config_json"].as_str().unwrap_or("{}"),
                w["icon"].as_str().unwrap_or(""),
                w["is_builtin"].as_i64().unwrap_or(0),
            ]).ok();
        }
    });
    eprintln!("[catalog] Loaded {} workflows", count);
}

// ──────────────────────────────────────────
// Fallback agents (if JSON not found)
// ──────────────────────────────────────────

fn seed_fallback_agents(cache: &mut HashMap<String, AgentInfo>) {
    let fallback = [
        ("rte-marie",     "Marie Lefevre",   "rte",           "Pragmatic RTE, coordinates the team"),
        ("po-lucas",      "Lucas Martin",    "product_owner", "Product Owner, prioritizes backlog"),
        ("archi-pierre",  "Pierre Garnier",  "architect",     "Solution architect, designs systems"),
        ("lead-thomas",   "Thomas Dubois",   "lead_dev",      "Lead developer, code quality guardian"),
        ("dev-emma",      "Clara Nguyen",    "developer",     "Frontend developer, React/TypeScript"),
        ("dev-karim",     "Karim Benali",    "developer",     "Backend developer, Rust/Python"),
        ("qa-sophie",     "Sophie Martin",   "qa",            "QA lead, testing specialist"),
    ];

    db::with_db(|conn| {
        for (id, name, role, desc) in &fallback {
            conn.execute(
                "INSERT OR IGNORE INTO agents (id, name, role, description, persona) VALUES (?1,?2,?3,?4,?4)",
                params![id, name, role, desc],
            ).ok();
            cache.insert(id.to_string(), AgentInfo {
                id: id.to_string(),
                name: name.to_string(),
                role: role.to_string(),
                persona: desc.to_string(),
                system_prompt: String::new(),
                avatar: String::new(),
                color: "#f78166".to_string(),
                tagline: String::new(),
                tools: vec![], skills: vec![],
                hierarchy_rank: 50,
                can_veto: false,
            });
        }
    });
    eprintln!("[catalog] Loaded {} fallback agents", fallback.len());
}

// ──────────────────────────────────────────
// Workflow lookup (from DB)
// ──────────────────────────────────────────

/// Get workflow phases from DB by workflow ID.
pub fn get_workflow_phases(id: &str) -> Option<Vec<(String, String, Vec<String>)>> {
    db::with_db(|conn| {
        let phases_json: Option<String> = conn.query_row(
            "SELECT phases_json FROM workflows WHERE id = ?1",
            params![id],
            |row| row.get(0),
        ).ok();

        phases_json.and_then(|pj| {
            let phases: Vec<Value> = serde_json::from_str(&pj).ok()?;
            let result: Vec<(String, String, Vec<String>)> = phases.iter().filter_map(|p| {
                let name = p.get("name").or_else(|| p.get("phase_name"))?.as_str()?.to_string();
                let pattern = p.get("pattern").and_then(|v| v.as_str()).unwrap_or("sequential").to_string();
                let agent_ids: Vec<String> = p.get("agent_ids")
                    .or_else(|| p.get("agents"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|a| {
                        a.as_str().map(|s| s.to_string())
                            .or_else(|| a.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    }).collect())
                    .unwrap_or_default();
                Some((name, pattern, agent_ids))
            }).collect();
            if result.is_empty() { None } else { Some(result) }
        })
    })
}

/// List all workflows from DB.
pub fn list_workflows() -> Vec<(String, String, String)> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description FROM workflows ORDER BY name"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        }).unwrap().filter_map(|r| r.ok()).collect()
    })
}

/// Catalog stats: (agents, skills, patterns, workflows)
pub fn catalog_stats() -> (usize, usize, usize, usize) {
    db::with_db(|conn| {
        let a: i64 = conn.query_row("SELECT COUNT(*) FROM agents", [], |r| r.get(0)).unwrap_or(0);
        let s: i64 = conn.query_row("SELECT COUNT(*) FROM skills", [], |r| r.get(0)).unwrap_or(0);
        let p: i64 = conn.query_row("SELECT COUNT(*) FROM patterns", [], |r| r.get(0)).unwrap_or(0);
        let w: i64 = conn.query_row("SELECT COUNT(*) FROM workflows", [], |r| r.get(0)).unwrap_or(0);
        (a as usize, s as usize, p as usize, w as usize)
    })
}
