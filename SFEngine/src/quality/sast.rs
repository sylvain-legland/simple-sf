// Ref: FT-SSF-026

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
            Self::Info => "INFO",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SASTFinding {
    pub file: String,
    pub line: usize,
    pub severity: Severity,
    pub rule: String,
    pub message: String,
}

/// Parse `cargo clippy --message-format=json` output into findings.
/// This provides the structure; actual execution is via shell.
pub fn run_clippy_check(clippy_json_output: &str) -> Vec<SASTFinding> {
    let mut findings = Vec::new();
    for line in clippy_json_output.lines() {
        // Each line is a JSON diagnostic from rustc/clippy
        if !line.contains("\"level\"") {
            continue;
        }
        let severity = if line.contains("\"error\"") {
            Severity::High
        } else if line.contains("\"warning\"") {
            Severity::Medium
        } else {
            continue;
        };
        // Extract message text (simple heuristic)
        let message = line
            .split("\"message\":\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("clippy finding")
            .to_string();
        findings.push(SASTFinding {
            file: String::new(),
            line: 0,
            severity,
            rule: "clippy".into(),
            message,
        });
    }
    findings
}

/// Custom static analysis rules applied to source content.
pub fn custom_rules(content: &str, filename: &str) -> Vec<SASTFinding> {
    let mut findings = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let lineno = i + 1;

        // Unwrap without context
        if line.contains(".unwrap()") && !line.contains("//") {
            findings.push(SASTFinding {
                file: filename.into(),
                line: lineno,
                severity: Severity::Medium,
                rule: "no-bare-unwrap".into(),
                message: ".unwrap() without context comment".into(),
            });
        }

        // Hardcoded secrets
        let lower = line.to_lowercase();
        if (lower.contains("password") || lower.contains("secret") || lower.contains("api_key"))
            && line.contains("= \"")
        {
            findings.push(SASTFinding {
                file: filename.into(),
                line: lineno,
                severity: Severity::Critical,
                rule: "hardcoded-secret".into(),
                message: "Possible hardcoded secret/credential".into(),
            });
        }

        // SQL injection risk
        if line.contains("format!") && (lower.contains("select") || lower.contains("insert") || lower.contains("delete")) {
            findings.push(SASTFinding {
                file: filename.into(),
                line: lineno,
                severity: Severity::High,
                rule: "sql-injection".into(),
                message: "format! with SQL — use parameterized queries".into(),
            });
        }

        // Path traversal
        if line.contains("..") && (line.contains("Path") || line.contains("open") || line.contains("read")) {
            findings.push(SASTFinding {
                file: filename.into(),
                line: lineno,
                severity: Severity::High,
                rule: "path-traversal".into(),
                message: "Potential path traversal with '..'".into(),
            });
        }
    }
    findings
}

pub fn format_report(findings: &[SASTFinding]) -> String {
    if findings.is_empty() {
        return "SAST: No findings.\n".into();
    }
    let mut out = format!("SAST: {} finding(s)\n", findings.len());
    for f in findings {
        out.push_str(&format!(
            "  [{}] {}:{} ({}) — {}\n",
            f.severity.as_str(),
            f.file,
            f.line,
            f.rule,
            f.message
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_bare_unwrap() {
        let code = "let x = foo.unwrap()\n";
        let findings = custom_rules(code, "test.rs");
        assert!(findings.iter().any(|f| f.rule == "no-bare-unwrap"));
    }

    #[test]
    fn detects_hardcoded_password() {
        let code = r#"let password = "hunter2";"#;
        let findings = custom_rules(code, "test.rs");
        assert!(findings.iter().any(|f| f.rule == "hardcoded-secret"));
    }

    #[test]
    fn detects_sql_injection() {
        let code = r#"let q = format!("SELECT * FROM users WHERE id = {}", id);"#;
        let findings = custom_rules(code, "test.rs");
        assert!(findings.iter().any(|f| f.rule == "sql-injection"));
    }

    #[test]
    fn clean_code_no_findings() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }\n";
        let findings = custom_rules(code, "clean.rs");
        assert!(findings.is_empty());
    }
}
