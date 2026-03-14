// Ref: FT-SSF-026
use std::cmp;

#[derive(Debug, Clone)]
pub struct ErrorCluster {
    pub representative: String,
    pub members: Vec<String>,
    pub count: usize,
}

pub fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] {
                0
            } else {
                1
            };
            curr[j] = cmp::min(
                cmp::min(prev[j] + 1, curr[j - 1] + 1),
                prev[j - 1] + cost,
            );
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_len]
}

pub fn similarity(a: &str, b: &str) -> f64 {
    let max_len = cmp::max(a.len(), b.len());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - (levenshtein(a, b) as f64 / max_len as f64)
}

pub fn cluster_errors(errors: &[String], threshold: f64) -> Vec<ErrorCluster> {
    let threshold = if threshold <= 0.0 || threshold > 1.0 {
        0.7
    } else {
        threshold
    };
    let mut clusters: Vec<ErrorCluster> = Vec::new();

    for error in errors {
        let mut matched = false;
        for cluster in clusters.iter_mut() {
            if similarity(&cluster.representative, error) >= threshold {
                cluster.members.push(error.clone());
                cluster.count += 1;
                matched = true;
                break;
            }
        }
        if !matched {
            clusters.push(ErrorCluster {
                representative: error.clone(),
                members: vec![error.clone()],
                count: 1,
            });
        }
    }
    clusters
}

pub fn format_clusters(clusters: &[ErrorCluster]) -> String {
    let mut out = format!("{} cluster(s) found:\n", clusters.len());
    for (i, c) in clusters.iter().enumerate() {
        out.push_str(&format!(
            "  [{}] ({} occurrence{}) {}\n",
            i + 1,
            c.count,
            if c.count > 1 { "s" } else { "" },
            c.representative
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_kitten_sitting() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn levenshtein_empty_strings() {
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("", ""), 0);
    }

    #[test]
    fn cluster_errors_groups_similar() {
        let errors = vec![
            "connection refused".into(),
            "connection reset".into(),
            "timeout expired".into(),
        ];
        let clusters = cluster_errors(&errors, 0.7);
        // "connection refused" and "connection reset" should cluster together
        assert!(clusters.len() <= 2);
    }
}
