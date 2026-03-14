// ══════════════════════════════════════════════════════════════
// INDEX WALKER — File walking, language detection, AST parsing
// ══════════════════════════════════════════════════════════════

use std::path::Path;
use std::fs;
use std::time::SystemTime;
use tree_sitter::{Node, Parser};

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
    if let Some(name_node) = node.child_by_field_name("name") {
        return node_text(&name_node, source);
    }
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

pub fn should_skip(path: &Path, workspace: &Path) -> bool {
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

pub fn file_mtime(path: &Path) -> u64 {
    path.metadata().ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
