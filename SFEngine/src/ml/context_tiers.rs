// Ref: FT-SSF-022
// Context Tiers L0/L1/L2 — adaptive context compression

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextTier {
    L0, // skeleton — minimal context
    L1, // summary — headers + first lines
    L2, // full content
}

pub fn tier_for_budget(budget: usize) -> ContextTier {
    if budget < 2000 {
        ContextTier::L0
    } else if budget < 8000 {
        ContextTier::L1
    } else {
        ContextTier::L2
    }
}

pub fn compress(content: &str, tier: ContextTier) -> String {
    match tier {
        ContextTier::L0 => {
            if content.len() <= 500 {
                content.to_string()
            } else {
                format!("{}...", &content[..500])
            }
        }
        ContextTier::L1 => {
            content
                .split("\n\n")
                .map(|paragraph| {
                    let trimmed = paragraph.trim();
                    if trimmed.starts_with('#') || trimmed.starts_with("//") || trimmed.is_empty() {
                        trimmed.to_string()
                    } else {
                        trimmed.lines().next().unwrap_or("").to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n\n")
        }
        ContextTier::L2 => content.to_string(),
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_for_budget_returns_correct_tier() {
        assert_eq!(tier_for_budget(500), ContextTier::L0);
        assert_eq!(tier_for_budget(4000), ContextTier::L1);
        assert_eq!(tier_for_budget(10000), ContextTier::L2);
    }

    #[test]
    fn compress_l0_truncates_long_content() {
        let long = "x".repeat(1000);
        let result = compress(&long, ContextTier::L0);
        assert!(result.len() < long.len());
        assert!(result.ends_with("..."));
    }

    #[test]
    fn compress_l2_returns_full() {
        let text = "Hello world\n\nSecond paragraph";
        assert_eq!(compress(text, ContextTier::L2), text);
    }
}
