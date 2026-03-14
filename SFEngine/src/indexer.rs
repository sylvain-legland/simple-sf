// ══════════════════════════════════════════════════════════════
// AST-BASED SEMANTIC CODE INDEXER
// Inspired by cocoindex-code — tree-sitter chunks + vector search
// ══════════════════════════════════════════════════════════════

mod index_store;
mod index_walker;

// Re-export public API
pub use index_store::{SearchResult, format_results, grep_search};
pub use index_walker::parse_file;
pub use index_walker::CodeChunk;

use std::path::Path;
use walkdir::WalkDir;
use index_store::{
    count_chunks, delete_stale_chunks, embed_batch,
    fts_search, get_indexed_mtimes, store_chunks, vector_search,
};
use index_walker::{file_mtime, should_skip};

// ─── Indexing pipeline ───────────────────────────────────────

// Ref: FT-SSF-022
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

    for (path, mtime) in &files {
        let rel = path.strip_prefix(ws).unwrap_or(path);
        let rel_str = rel.to_string_lossy().to_string();
        if let Some(&old_mtime) = existing_mtimes.get(&rel_str) {
            if *mtime == old_mtime {
                continue;
            }
        }
        changed_files.push((path.clone(), *mtime));
    }

    // If nothing changed and we have existing chunks, skip
    if changed_files.is_empty() && !existing_mtimes.is_empty() {
        return Ok((count_chunks(workspace), false));
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

    delete_stale_chunks(workspace, &changed_files, &current_rel_paths, ws);

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

    store_chunks(workspace, &new_chunks, embeddings.as_ref());

    Ok((count_chunks(workspace), has_embeddings))
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
    if let Ok(query_emb) = crate::llm::embed(&[query]).await {
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
