// Ref: FT-SSF-024
//! Clean Architecture — layer dependency validation

#[derive(Debug, Clone, PartialEq)]
pub enum Layer {
    Domain,
    Application,
    Infrastructure,
    Presentation,
}

pub struct ModuleMapping {
    pub module: String,
    pub layer: Layer,
}

pub fn default_mappings() -> Vec<ModuleMapping> {
    let map = |m: &str, l: Layer| ModuleMapping { module: m.to_string(), layer: l };
    vec![
        map("engine/types", Layer::Domain),
        map("engine/mission", Layer::Domain),
        map("agents", Layer::Domain),
        map("guard", Layer::Domain),
        map("engine/patterns", Layer::Application),
        map("engine/workflow", Layer::Application),
        map("engine/phase", Layer::Application),
        map("ml", Layer::Application),
        map("db", Layer::Infrastructure),
        map("llm", Layer::Infrastructure),
        map("tools", Layer::Infrastructure),
        map("executor", Layer::Infrastructure),
        map("sandbox", Layer::Infrastructure),
        map("ffi", Layer::Presentation),
    ]
}

/// Returns true if the dependency direction is allowed.
pub fn check_dependencies(from: &Layer, to: &Layer) -> bool {
    match (from, to) {
        (_, Layer::Domain) => true,
        (Layer::Presentation, Layer::Application) => true,
        (Layer::Application, Layer::Infrastructure) => false,
        (Layer::Domain, Layer::Infrastructure | Layer::Presentation) => false,
        (Layer::Infrastructure, Layer::Application | Layer::Presentation) => false,
        _ => true,
    }
}

/// Scan a source string for `use crate::` imports and warn on layer violations.
#[allow(dead_code)]
pub fn validate_imports(file: &str, module_layer: &Layer) -> Vec<String> {
    let mappings = default_mappings();
    let mut warnings = Vec::new();
    for line in file.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("use crate::") {
            for m in &mappings {
                if rest.starts_with(&m.module) && !check_dependencies(module_layer, &m.layer) {
                    warnings.push(format!(
                        "Layer violation: {:?} -> {:?} via `{}`", module_layer, m.layer, trimmed
                    ));
                }
            }
        }
    }
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_to_infra_forbidden() {
        assert!(!check_dependencies(&Layer::Domain, &Layer::Infrastructure));
    }

    #[test]
    fn application_to_domain_allowed() {
        assert!(check_dependencies(&Layer::Application, &Layer::Domain));
    }

    #[test]
    fn presentation_to_application_allowed() {
        assert!(check_dependencies(&Layer::Presentation, &Layer::Application));
    }
}
