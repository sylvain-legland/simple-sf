// Ref: FT-SSF-023

#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: String,
    pub given: Vec<String>,
    pub when: Vec<String>,
    pub then: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Feature {
    pub name: String,
    pub description: String,
    pub scenarios: Vec<Scenario>,
}

pub fn parse_feature(content: &str) -> Result<Feature, String> {
    let mut name = String::new();
    let mut description = String::new();
    let mut scenarios: Vec<Scenario> = Vec::new();
    let mut current: Option<Scenario> = None;
    let mut last_section: Option<&str> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("Feature:") {
            name = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("Scenario:") {
            if let Some(s) = current.take() {
                scenarios.push(s);
            }
            current = Some(Scenario {
                name: rest.trim().to_string(),
                given: Vec::new(),
                when: Vec::new(),
                then: Vec::new(),
            });
            last_section = None;
        } else if let Some(rest) = line.strip_prefix("Given ") {
            if let Some(ref mut s) = current {
                s.given.push(rest.to_string());
                last_section = Some("given");
            }
        } else if let Some(rest) = line.strip_prefix("When ") {
            if let Some(ref mut s) = current {
                s.when.push(rest.to_string());
                last_section = Some("when");
            }
        } else if let Some(rest) = line.strip_prefix("Then ") {
            if let Some(ref mut s) = current {
                s.then.push(rest.to_string());
                last_section = Some("then");
            }
        } else if let Some(rest) = line.strip_prefix("And ") {
            if let Some(ref mut s) = current {
                match last_section {
                    Some("given") => s.given.push(rest.to_string()),
                    Some("when") => s.when.push(rest.to_string()),
                    Some("then") => s.then.push(rest.to_string()),
                    _ => {}
                }
            }
        } else if !line.is_empty() && name.is_empty() == false && scenarios.is_empty() && current.is_none() {
            if !description.is_empty() {
                description.push(' ');
            }
            description.push_str(line);
        }
    }

    if let Some(s) = current {
        scenarios.push(s);
    }

    if name.is_empty() {
        return Err("Missing Feature: line".to_string());
    }

    Ok(Feature { name, description, scenarios })
}

pub fn format_feature(feature: &Feature) -> String {
    let mut out = format!("Feature: {}\n", feature.name);
    if !feature.description.is_empty() {
        out.push_str(&format!("  {}\n", feature.description));
    }
    out.push('\n');
    for scenario in &feature.scenarios {
        out.push_str(&format!("  Scenario: {}\n", scenario.name));
        for (i, g) in scenario.given.iter().enumerate() {
            let kw = if i == 0 { "Given" } else { "And" };
            out.push_str(&format!("    {} {}\n", kw, g));
        }
        for (i, w) in scenario.when.iter().enumerate() {
            let kw = if i == 0 { "When" } else { "And" };
            out.push_str(&format!("    {} {}\n", kw, w));
        }
        for (i, t) in scenario.then.iter().enumerate() {
            let kw = if i == 0 { "Then" } else { "And" };
            out.push_str(&format!("    {} {}\n", kw, t));
        }
        out.push('\n');
    }
    out
}

pub fn validate_scenario(s: &Scenario) -> Vec<String> {
    let mut errors = Vec::new();
    if s.given.is_empty() {
        errors.push(format!("Scenario '{}': missing Given step", s.name));
    }
    if s.when.is_empty() {
        errors.push(format!("Scenario '{}': missing When step", s.name));
    }
    if s.then.is_empty() {
        errors.push(format!("Scenario '{}': missing Then step", s.name));
    }
    errors
}
