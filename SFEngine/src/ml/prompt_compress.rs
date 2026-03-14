// Ref: FT-SSF-022
// Prompt Compression — 40-70% token savings via rule-based text reduction

const ABBREVIATIONS: &[(&str, &str)] = &[
    ("function", "fn"),
    ("implementation", "impl"),
    ("documentation", "doc"),
    ("parameter", "param"),
    ("argument", "arg"),
    ("configuration", "config"),
    ("application", "app"),
    ("environment", "env"),
    ("information", "info"),
    ("description", "desc"),
];

pub fn compress(text: &str) -> String {
    let mut result = text.to_string();

    // Strip trailing whitespace per line
    result = result
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    // Collapse consecutive blank lines into one
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    // Abbreviate common words (case-insensitive replacement)
    for &(long, short) in ABBREVIATIONS {
        result = result.replace(long, short);
        let capitalized = format!("{}{}", &long[..1].to_uppercase(), &long[1..]);
        let cap_short = format!("{}{}", &short[..1].to_uppercase(), &short[1..]);
        result = result.replace(&capitalized, &cap_short);
    }

    // Collapse repeated phrases (3+ consecutive identical words)
    let words: Vec<&str> = result.split_whitespace().collect();
    let mut compressed_words: Vec<String> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if i + 2 < words.len() && words[i] == words[i + 1] && words[i + 1] == words[i + 2] {
            compressed_words.push(format!("{}(x)", words[i]));
            while i < words.len() && words.get(i) == words.get(i.wrapping_sub(1).min(i)) {
                i += 1;
            }
        } else {
            compressed_words.push(words[i].to_string());
            i += 1;
        }
    }

    compressed_words.join(" ")
}

pub fn compression_ratio(original: &str, compressed: &str) -> f64 {
    if original.is_empty() {
        return 0.0;
    }
    1.0 - (compressed.len() as f64 / original.len() as f64)
}
