// Ref: FT-SSF-023

#[derive(Debug, Clone)]
pub struct YAGNIReport {
    pub filename: String,
    pub dead_imports: Vec<String>,
    pub todo_count: usize,
    pub unused_fns: Vec<String>,
    pub complexity_warnings: Vec<String>,
}

pub fn analyze_file(content: &str, filename: &str) -> YAGNIReport {
    let mut dead_imports = Vec::new();
    let mut unused_fns = Vec::new();
    let mut complexity_warnings = Vec::new();
    let mut todo_count = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.contains("#[allow(dead_code)]") {
            dead_imports.push(trimmed.to_string());
        }

        if trimmed.contains("// TODO") || trimmed.contains("// FIXME") {
            todo_count += 1;
        }

        if trimmed.starts_with("fn _") || trimmed.starts_with("pub fn _") {
            let name = trimmed.split('(').next().unwrap_or(trimmed);
            unused_fns.push(name.trim().to_string());
        }
    }

    let line_count = content.lines().count();
    if line_count > 300 {
        complexity_warnings.push(format!(
            "{}: {} lines (recommended max 300)",
            filename, line_count
        ));
    }

    YAGNIReport {
        filename: filename.to_string(),
        dead_imports,
        todo_count,
        unused_fns,
        complexity_warnings,
    }
}

pub fn format_report(reports: &[YAGNIReport]) -> String {
    let mut out = String::from("=== YAGNI Report ===\n\n");
    for r in reports {
        out.push_str(&format!("File: {}\n", r.filename));
        out.push_str(&format!("  TODO/FIXME: {}\n", r.todo_count));
        if !r.dead_imports.is_empty() {
            out.push_str(&format!("  Dead code attrs: {}\n", r.dead_imports.len()));
        }
        if !r.unused_fns.is_empty() {
            out.push_str(&format!("  Unused fns: {:?}\n", r.unused_fns));
        }
        for w in &r.complexity_warnings {
            out.push_str(&format!("  ⚠ {}\n", w));
        }
        out.push('\n');
    }
    out
}
