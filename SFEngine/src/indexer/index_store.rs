// ══════════════════════════════════════════════════════════════
// INDEX STORE — Storage, retrieval, and search
// ══════════════════════════════════════════════════════════════

use crate::db;
use crate::llm;
use super::index_walker::CodeChunk;

// ─── Search result type ──────────────────────────────────────

// Ref: FT-SSF-022
pub struct SearchResult {
    pub file_path: String,
    pub language: String,
    pub chunk_type: String,
    pub name: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub score: f64,
}

// ─── DB helpers ──────────────────────────────────────────────

pub fn get_indexed_mtimes(workspace: &str) -> std::collections::HashMap<String, u64> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT file_path, file_mtime FROM code_chunks WHERE workspace = ?1 GROUP BY file_path"
        ).unwrap();
        let rows = stmt.query_map(rusqlite::params![workspace], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?))
        }).unwrap();
        rows.filter_map(|r| r.ok()).collect()
    })
}

pub fn count_chunks(workspace: &str) -> usize {
    db::with_db(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM code_chunks WHERE workspace = ?1",
            rusqlite::params![workspace], |r| r.get::<_, i64>(0)
        ).unwrap_or(0) as usize
    })
}

pub fn delete_stale_chunks(
    workspace: &str,
    changed_files: &[(std::path::PathBuf, u64)],
    current_rel_paths: &std::collections::HashSet<String>,
    ws: &std::path::Path,
) {
    db::with_db(|conn| {
        for (path, _) in changed_files {
            let rel = path.strip_prefix(ws).unwrap_or(path).to_string_lossy().to_string();
            conn.execute(
                "DELETE FROM code_chunks WHERE workspace = ?1 AND file_path = ?2",
                rusqlite::params![workspace, rel],
            ).ok();
        }
        let mut stmt = conn.prepare(
            "SELECT DISTINCT file_path FROM code_chunks WHERE workspace = ?1"
        ).unwrap();
        let indexed_paths: Vec<String> = stmt.query_map(
            rusqlite::params![workspace], |r| r.get(0)
        ).unwrap().filter_map(|r| r.ok()).collect();

        for path in indexed_paths {
            if !current_rel_paths.contains(&path) {
                conn.execute(
                    "DELETE FROM code_chunks WHERE workspace = ?1 AND file_path = ?2",
                    rusqlite::params![workspace, path],
                ).ok();
            }
        }
    });
}

pub fn store_chunks(
    workspace: &str,
    new_chunks: &[(CodeChunk, u64)],
    embeddings: Option<&Vec<Vec<f32>>>,
) {
    db::with_db(|conn| {
        for (i, (chunk, mtime)) in new_chunks.iter().enumerate() {
            let embedding_blob: Option<Vec<u8>> = embeddings.and_then(|embs| {
                embs.get(i).map(|v| {
                    v.iter().flat_map(|f| f.to_le_bytes()).collect()
                })
            });

            conn.execute(
                "INSERT INTO code_chunks (workspace, file_path, language, chunk_type, name, content, start_line, end_line, embedding, file_mtime)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    workspace, chunk.file_path, chunk.language, chunk.chunk_type,
                    chunk.name, chunk.content, chunk.start_line, chunk.end_line,
                    embedding_blob, mtime,
                ],
            ).ok();

            // Insert into FTS5
            let rowid = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO code_chunks_fts (rowid, name, content) VALUES (?1, ?2, ?3)",
                rusqlite::params![rowid, chunk.name, chunk.content],
            ).ok();
        }
    });
}

pub async fn embed_batch(texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
    let mut all = Vec::new();
    // Batch 16 at a time (embedding API limits)
    for batch in texts.chunks(16) {
        let embeddings = llm::embed(batch).await?;
        all.extend(embeddings);
    }
    Ok(all)
}

// ─── Search ──────────────────────────────────────────────────

/// Regex search (grep-like) — kept for backward compat
pub fn grep_search(pattern: &str, workspace: &str, limit: usize) -> Vec<SearchResult> {
    let re = match regex::Regex::new(pattern) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT file_path, language, chunk_type, name, content, start_line, end_line
             FROM code_chunks WHERE workspace = ?1"
        ).unwrap();

        let mut results = Vec::new();
        let rows = stmt.query_map(rusqlite::params![workspace], |r| {
            Ok((
                r.get::<_, String>(0)?, r.get::<_, String>(1)?,
                r.get::<_, String>(2)?, r.get::<_, String>(3)?,
                r.get::<_, String>(4)?, r.get::<_, i64>(5)? as usize,
                r.get::<_, i64>(6)? as usize,
            ))
        }).unwrap();

        for row in rows.filter_map(|r| r.ok()) {
            let (file_path, language, chunk_type, name, content, start_line, end_line) = row;
            if re.is_match(&content) || re.is_match(&name) {
                results.push(SearchResult {
                    file_path, language, chunk_type, name,
                    content, start_line, end_line, score: 1.0,
                });
                if results.len() >= limit { break; }
            }
        }
        results
    })
}

pub fn vector_search(query_emb: &[f32], workspace: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT file_path, language, chunk_type, name, content, start_line, end_line, embedding
             FROM code_chunks WHERE workspace = ?1 AND embedding IS NOT NULL"
        ).map_err(|e| e.to_string())?;

        let mut scored: Vec<SearchResult> = Vec::new();

        let rows = stmt.query_map(rusqlite::params![workspace], |r| {
            let embedding_blob: Vec<u8> = r.get(7)?;
            let embedding: Vec<f32> = embedding_blob.chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            let score = cosine_similarity(query_emb, &embedding);

            Ok(SearchResult {
                file_path: r.get(0)?,
                language: r.get(1)?,
                chunk_type: r.get(2)?,
                name: r.get(3)?,
                content: r.get(4)?,
                start_line: r.get::<_, i64>(5)? as usize,
                end_line: r.get::<_, i64>(6)? as usize,
                score,
            })
        }).map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok(r) = row { scored.push(r); }
        }

        // Sort by descending score, take top-K
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Filter low-relevance (score < 0.3)
        scored.retain(|r| r.score > 0.3);

        Ok(scored)
    })
}

pub fn fts_search(query: &str, workspace: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    // Sanitize query for FTS5 (remove special chars that break MATCH)
    let sanitized: String = query.chars()
        .map(|c| if c.is_alphanumeric() || c == ' ' || c == '_' { c } else { ' ' })
        .collect();
    let terms: Vec<&str> = sanitized.split_whitespace().collect();
    if terms.is_empty() { return Ok(vec![]); }

    // FTS5 query: OR between terms for broader match
    let fts_query = terms.join(" OR ");

    db::with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT c.file_path, c.language, c.chunk_type, c.name, c.content,
                    c.start_line, c.end_line, bm25(code_chunks_fts) as score
             FROM code_chunks_fts f
             JOIN code_chunks c ON c.rowid = f.rowid
             WHERE code_chunks_fts MATCH ?1 AND c.workspace = ?2
             ORDER BY score
             LIMIT ?3"
        ).map_err(|e| e.to_string())?;

        let results: Vec<SearchResult> = stmt.query_map(
            rusqlite::params![fts_query, workspace, limit],
            |r| Ok(SearchResult {
                file_path: r.get(0)?,
                language: r.get(1)?,
                chunk_type: r.get(2)?,
                name: r.get(3)?,
                content: r.get(4)?,
                start_line: r.get::<_, i64>(5)? as usize,
                end_line: r.get::<_, i64>(6)? as usize,
                score: r.get::<_, f64>(7)?.abs(), // BM25 returns negative scores
            })
        ).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(results)
    })
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let (mut dot, mut na, mut nb) = (0.0_f64, 0.0_f64, 0.0_f64);
    for (x, y) in a.iter().zip(b.iter()) {
        let (xf, yf) = (*x as f64, *y as f64);
        dot += xf * yf;
        na += xf * xf;
        nb += yf * yf;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 { 0.0 } else { dot / denom }
}

// ─── Format for agent output ────────────────────────────────

pub fn format_results(results: &[SearchResult]) -> String {
    if results.is_empty() {
        return "No results found.".to_string();
    }
    let mut out = String::new();
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!(
            "── {} ── [{}] {}/{} (L{}-L{}) score:{:.2}\n{}\n\n",
            i + 1, r.language, r.file_path, r.name,
            r.start_line, r.end_line, r.score,
            // Truncate content for display
            if r.content.len() > 1500 { &r.content[..r.content.floor_char_boundary(1500)] } else { &r.content }
        ));
    }
    out
}
