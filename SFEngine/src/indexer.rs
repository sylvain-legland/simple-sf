// ══════════════════════════════════════════════════════════════
// AST-BASED SEMANTIC CODE INDEXER
// Inspired by cocoindex-code — tree-sitter chunks + vector search
// ══════════════════════════════════════════════════════════════

use std::path::Path;
use std::fs;
use std::time::SystemTime;
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;
use crate::db;
use crate::llm;

// ─── Data types ──────────────────────────────────────────────

// Ref: FT-SSF-022
pub struct CodeChunk {
    pub file_path: String,
    pub language: String,
    pub chunk_type: String,
    pub name: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
}

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

// ─── Language detection ──────────────────────────────────────

fn detect_language(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()? {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "jsx" | "mjs" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "go" => Some("go"),
        "c" | "h" => Some("c"),
        "swift" => Some("swift"),
        "java" => Some("java"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
        "css" | "scss" => Some("css"),
        "html" | "htm" => Some("html"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        "md" | "mdx" => Some("markdown"),
        "sql" => Some("sql"),
        "sh" | "bash" | "zsh" => Some("shell"),
        _ => None,
    }
}

fn get_parser(language: &str) -> Option<Parser> {
    let mut parser = Parser::new();
    let ok = match language {
        "rust"       => parser.set_language(&tree_sitter_rust::LANGUAGE.into()),
        "python"     => parser.set_language(&tree_sitter_python::LANGUAGE.into()),
        "javascript" => parser.set_language(&tree_sitter_javascript::LANGUAGE.into()),
        "typescript" => parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "go"         => parser.set_language(&tree_sitter_go::LANGUAGE.into()),
        "c" | "cpp"  => parser.set_language(&tree_sitter_c::LANGUAGE.into()),
        "swift"      => parser.set_language(&tree_sitter_swift::LANGUAGE.into()),
        _ => return None,
    };
    ok.ok()?;
    Some(parser)
}

// AST node types to extract per language
fn is_target_node(kind: &str, language: &str) -> bool {
    match language {
        "rust" => matches!(kind,
            "function_item" | "struct_item" | "enum_item" | "impl_item" |
            "trait_item" | "mod_item" | "const_item" | "static_item" | "type_item"
        ),
        "python" => matches!(kind, "function_definition" | "class_definition"),
        "javascript" | "typescript" => matches!(kind,
            "function_declaration" | "class_declaration" | "method_definition" |
            "arrow_function" | "export_statement" | "lexical_declaration"
        ),
        "go" => matches!(kind,
            "function_declaration" | "method_declaration" | "type_declaration" |
            "const_declaration" | "var_declaration"
        ),
        "c" | "cpp" => matches!(kind,
            "function_definition" | "struct_specifier" | "enum_specifier" |
            "declaration" | "preproc_function_def"
        ),
        "swift" => matches!(kind,
            "function_declaration" | "class_declaration" | "struct_declaration" |
            "enum_declaration" | "protocol_declaration" | "extension_declaration"
        ),
        _ => false,
    }
}

// ─── AST chunking ────────────────────────────────────────────

fn extract_name(node: &Node, source: &[u8]) -> String {
    // Try field "name" first (most languages)
    if let Some(name_node) = node.child_by_field_name("name") {
        return node_text(&name_node, source);
    }
    // Scan children for identifier-like nodes
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "type_identifier" | "field_identifier" |
            "property_identifier" | "simple_identifier" => {
                return node_text(&child, source);
            }
            _ => {}
        }
    }
    format!("anon_{}", node.kind())
}

fn node_text(node: &Node, source: &[u8]) -> String {
    String::from_utf8_lossy(&source[node.start_byte()..node.end_byte()]).to_string()
}

fn walk_tree(
    node: Node, source: &[u8], language: &str, file_path: &str,
    chunks: &mut Vec<CodeChunk>,
) {
    if is_target_node(node.kind(), language) {
        let content = node_text(&node, source);
        // Skip tiny chunks (< 2 lines) — likely noise
        if content.lines().count() >= 2 {
            let name = extract_name(&node, source);
            chunks.push(CodeChunk {
                file_path: file_path.to_string(),
                language: language.to_string(),
                chunk_type: node.kind().to_string(),
                name,
                content: truncate(&content, 4000),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
            });
        }
        return; // Don't recurse into extracted nodes
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_tree(child, source, language, file_path, chunks);
    }
}

/// Parse a single file into AST-based code chunks.
pub fn parse_file(path: &Path, workspace: &str) -> Vec<CodeChunk> {
    let language = match detect_language(path) {
        Some(l) => l,
        None => return vec![],
    };

    let source = match fs::read(path) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    // Skip binary / very large files
    if source.len() > 200_000 { return vec![]; }

    let rel_path = path.strip_prefix(workspace).unwrap_or(path);
    let file_path = rel_path.to_string_lossy().to_string();

    // Try tree-sitter parsing
    if let Some(mut parser) = get_parser(language) {
        if let Some(tree) = parser.parse(&source, None) {
            let mut chunks = Vec::new();
            walk_tree(tree.root_node(), &source, language, &file_path, &mut chunks);

            if !chunks.is_empty() {
                return chunks;
            }
        }
    }

    // Fallback: treat whole file as one chunk (for languages without grammar)
    let text = String::from_utf8_lossy(&source);
    if text.trim().is_empty() { return vec![]; }

    vec![CodeChunk {
        file_path,
        language: language.to_string(),
        chunk_type: "file".to_string(),
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        content: truncate(&text, 4000),
        start_line: 1,
        end_line: text.lines().count(),
    }]
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() }
    else {
        // Cut at char boundary
        let end = s.floor_char_boundary(max);
        s[..end].to_string()
    }
}

// ─── Skip patterns ──────────────────────────────────────────

const SKIP_DIRS: &[&str] = &[
    ".git", "node_modules", "target", "dist", "build", "__pycache__",
    ".build", ".swiftpm", "vendor", "Pods", ".next", "coverage",
    "venv", ".venv", "env", ".tox", "eggs", ".eggs",
];

const SKIP_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "svg", "ico", "webp",
    "woff", "woff2", "ttf", "eot",
    "mp3", "mp4", "wav", "avi", "mov",
    "pdf", "zip", "tar", "gz", "bz2", "xz",
    "lock", "map", "min.js", "min.css",
    "o", "a", "so", "dylib", "dll", "exe",
    "pyc", "pyo", "class", "jar",
];

fn should_skip(path: &Path, workspace: &Path) -> bool {
    let rel = path.strip_prefix(workspace).unwrap_or(path);

    for component in rel.components() {
        if let Some(s) = component.as_os_str().to_str() {
            if SKIP_DIRS.contains(&s) { return true; }
            if s.starts_with('.') && s != ".gitignore" && s != ".env" { return true; }
        }
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if SKIP_EXTENSIONS.contains(&ext) { return true; }
    }

    false
}

// ─── Indexing pipeline ───────────────────────────────────────

fn file_mtime(path: &Path) -> u64 {
    path.metadata().ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Index all source files in a workspace. Returns (chunks_count, has_embeddings).
pub async fn index_workspace(workspace: &str) -> Result<(usize, bool), String> {
    let ws = Path::new(workspace);
    if !ws.is_dir() {
        return Err(format!("Workspace not found: {}", workspace));
    }

    // Collect all source files with mtimes
    let mut files: Vec<(std::path::PathBuf, u64)> = Vec::new();
    for entry in WalkDir::new(ws).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path().to_path_buf();
        if !path.is_file() || should_skip(&path, ws) { continue; }
        let mtime = file_mtime(&path);
        files.push((path, mtime));
    }

    // Check which files changed since last index
    let existing_mtimes = get_indexed_mtimes(workspace);
    let mut changed_files = Vec::new();
    let mut unchanged_paths: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (path, mtime) in &files {
        let rel = path.strip_prefix(ws).unwrap_or(path);
        let rel_str = rel.to_string_lossy().to_string();
        if let Some(&old_mtime) = existing_mtimes.get(&rel_str) {
            if *mtime == old_mtime {
                unchanged_paths.insert(rel_str);
                continue;
            }
        }
        changed_files.push((path.clone(), *mtime));
    }

    // If nothing changed and we have existing chunks, skip
    if changed_files.is_empty() && !existing_mtimes.is_empty() {
        let count = db::with_db(|conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM code_chunks WHERE workspace = ?1",
                rusqlite::params![workspace], |r| r.get::<_, i64>(0)
            ).unwrap_or(0) as usize
        });
        return Ok((count, false));
    }

    // Parse changed files
    let mut new_chunks: Vec<(CodeChunk, u64)> = Vec::new();
    for (path, mtime) in &changed_files {
        let chunks = parse_file(path, workspace);
        for chunk in chunks {
            new_chunks.push((chunk, *mtime));
        }
    }

    // Delete stale chunks (changed files + removed files)
    let current_rel_paths: std::collections::HashSet<String> = files.iter()
        .map(|(p, _)| p.strip_prefix(ws).unwrap_or(p).to_string_lossy().to_string())
        .collect();

    db::with_db(|conn| {
        // Delete chunks for changed files
        for (path, _) in &changed_files {
            let rel = path.strip_prefix(ws).unwrap_or(path).to_string_lossy().to_string();
            conn.execute(
                "DELETE FROM code_chunks WHERE workspace = ?1 AND file_path = ?2",
                rusqlite::params![workspace, rel],
            ).ok();
        }
        // Delete chunks for removed files
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

    // Try to compute embeddings for new chunks
    let texts: Vec<&str> = new_chunks.iter().map(|(c, _)| c.content.as_str()).collect();
    let embeddings = if !texts.is_empty() {
        match embed_batch(&texts).await {
            Ok(e) => Some(e),
            Err(_) => None,
        }
    } else {
        None
    };

    let has_embeddings = embeddings.is_some();

    // Store new chunks
    db::with_db(|conn| {
        for (i, (chunk, mtime)) in new_chunks.iter().enumerate() {
            let embedding_blob: Option<Vec<u8>> = embeddings.as_ref().and_then(|embs| {
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

    // Total count
    let total = db::with_db(|conn| {
        conn.query_row(
            "SELECT COUNT(*) FROM code_chunks WHERE workspace = ?1",
            rusqlite::params![workspace], |r| r.get::<_, i64>(0)
        ).unwrap_or(0) as usize
    });

    Ok((total, has_embeddings))
}

fn get_indexed_mtimes(workspace: &str) -> std::collections::HashMap<String, u64> {
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

async fn embed_batch(texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
    let mut all = Vec::new();
    // Batch 16 at a time (embedding API limits)
    for batch in texts.chunks(16) {
        let embeddings = llm::embed(batch).await?;
        all.extend(embeddings);
    }
    Ok(all)
}

// ─── Search ──────────────────────────────────────────────────

/// Semantic search: tries vector similarity first, falls back to FTS5.
pub async fn search(query: &str, workspace: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    // Ensure workspace is indexed
    let (count, _) = index_workspace(workspace).await?;
    if count == 0 {
        return Ok(vec![]);
    }

    // Try vector search
    if let Ok(query_emb) = llm::embed(&[query]).await {
        if let Some(qe) = query_emb.into_iter().next() {
            let results = vector_search(&qe, workspace, limit)?;
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }

    // Fallback: FTS5 full-text search
    fts_search(query, workspace, limit)
}

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

fn vector_search(query_emb: &[f32], workspace: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
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

fn fts_search(query: &str, workspace: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
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
