// Ref: FT-SSF-022
// Local vector similarity — bag-of-words embeddings and cosine search

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

const EMBED_DIM: usize = 64;

pub struct VectorStore {
    pub vectors: Vec<(String, Vec<f64>)>,
}

impl VectorStore {
    pub fn new() -> Self {
        Self { vectors: Vec::new() }
    }

    pub fn add(&mut self, id: String, embedding: Vec<f64>) {
        self.vectors.push((id, embedding));
    }

    pub fn search(&self, query: &[f64], top_k: usize) -> Vec<(String, f64)> {
        let mut scored: Vec<(String, f64)> = self.vectors.iter()
            .map(|(id, vec)| (id.clone(), cosine_similarity(query, vec)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

pub fn simple_embed(text: &str) -> Vec<f64> {
    let mut vec = vec![0.0_f64; EMBED_DIM];
    let lowered = text.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    for word in &words {
        let mut hasher = DefaultHasher::new();
        word.hash(&mut hasher);
        let h = hasher.finish();
        let idx = (h as usize) % EMBED_DIM;
        let sign = if (h >> 32) & 1 == 0 { 1.0 } else { -1.0 };
        vec[idx] += sign;
    }
    // L2 normalize
    let norm: f64 = vec.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm > 0.0 {
        for v in &mut vec {
            *v /= norm;
        }
    }
    vec
}
