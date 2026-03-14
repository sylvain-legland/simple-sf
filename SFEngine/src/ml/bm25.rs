// Ref: FT-SSF-022
// BM25 ranking for tool selection — tf-idf variant with length normalization

pub struct BM25Ranker {
    pub k1: f64,
    pub b: f64,
    pub avg_dl: f64,
    pub corpus: Vec<(String, Vec<String>)>,
}

impl BM25Ranker {
    pub fn new(k1: f64, b: f64) -> Self {
        Self { k1, b, avg_dl: 0.0, corpus: Vec::new() }
    }

    pub fn add_document(&mut self, id: String, text: &str) {
        let terms: Vec<String> = tokenize(text);
        self.corpus.push((id, terms));
        self.avg_dl = self.corpus.iter().map(|(_, t)| t.len() as f64).sum::<f64>()
            / self.corpus.len() as f64;
    }

    pub fn rank(&self, query: &str) -> Vec<(String, f64)> {
        let query_terms = tokenize(query);
        let n = self.corpus.len() as f64;
        let mut scores: Vec<(String, f64)> = self.corpus.iter().map(|(id, terms)| {
            let dl = terms.len() as f64;
            let score: f64 = query_terms.iter().map(|qt| {
                let tf = terms.iter().filter(|t| t == &qt).count() as f64;
                let df = self.corpus.iter().filter(|(_, ts)| ts.contains(qt)).count() as f64;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                idf * (tf * (self.k1 + 1.0)) / (tf + self.k1 * (1.0 - self.b + self.b * dl / self.avg_dl))
            }).sum();
            (id.clone(), score)
        }).collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    pub fn select_tools(&self, query: &str, _tools: &[(String, String)]) -> Vec<String> {
        let ranked = self.rank(query);
        let threshold = ranked.first().map(|(_, s)| s * 0.5).unwrap_or(0.0);
        ranked.into_iter()
            .filter(|(_, s)| *s > threshold && *s > 0.0)
            .map(|(id, _)| id)
            .collect()
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(String::from)
        .collect()
}

pub fn create_default() -> BM25Ranker {
    BM25Ranker::new(1.5, 0.75)
}
