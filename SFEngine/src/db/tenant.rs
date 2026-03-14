// Ref: FT-SSF-026
use std::fs;
use std::path::Path;

pub struct TenantContext {
    pub tenant_id: String,
    pub db_path: String,
    pub workspace: String,
}

impl TenantContext {
    pub fn for_project(project_id: &str, base_dir: &str) -> TenantContext {
        TenantContext {
            tenant_id: project_id.to_string(),
            db_path: format!("{}/data/{}.db", base_dir, project_id),
            workspace: format!("{}/workspaces/{}/", base_dir, project_id),
        }
    }

    pub fn ensure_dirs(&self) -> Result<(), String> {
        let db_parent = Path::new(&self.db_path)
            .parent()
            .ok_or("Invalid db_path")?;
        fs::create_dir_all(db_parent).map_err(|e| e.to_string())?;
        fs::create_dir_all(&self.workspace).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn db_path(&self) -> &str {
        &self.db_path
    }

    pub fn workspace_path(&self) -> &str {
        &self.workspace
    }
}

/// Prepend a tenant prefix to table names in a SQL query.
/// Simple token replacement: replaces `FROM {table}` and `INTO {table}`
/// with the prefixed version.
pub fn scoped_query(prefix: &str, query: &str) -> String {
    if prefix.is_empty() {
        return query.to_string();
    }
    let prefixed = format!("{}_", prefix);
    let mut result = query.to_string();
    for keyword in &["FROM ", "INTO ", "UPDATE ", "TABLE "] {
        let upper = *keyword;
        let lower = keyword.to_lowercase();
        // Handle both cases
        for kw in &[upper.to_string(), lower] {
            if let Some(pos) = result.find(kw.as_str()) {
                let after = pos + kw.len();
                if after < result.len() {
                    result.insert_str(after, &prefixed);
                }
            }
        }
    }
    result
}
