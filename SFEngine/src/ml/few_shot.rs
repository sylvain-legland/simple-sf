// Ref: FT-SSF-022
// Few-shot example injection — select and format task-relevant examples

pub struct FewShotBank {
    pub examples: Vec<Example>,
}

pub struct Example {
    pub task_type: String,
    pub input: String,
    pub output: String,
    pub quality: f64,
}

impl FewShotBank {
    pub fn new() -> Self {
        Self { examples: Vec::new() }
    }

    pub fn add_example(&mut self, example: Example) {
        self.examples.push(example);
    }

    pub fn select_examples(&self, task: &str, n: usize) -> Vec<&Example> {
        let task_lower = task.to_lowercase();
        let task_words: Vec<&str> = task_lower.split_whitespace().collect();

        let mut scored: Vec<(&Example, f64)> = self.examples.iter().map(|ex| {
            let ex_text = format!("{} {}", ex.task_type, ex.input).to_lowercase();
            let keyword_hits = task_words.iter()
                .filter(|w| w.len() > 2 && ex_text.contains(*w))
                .count() as f64;
            let score = keyword_hits * ex.quality;
            (ex, score)
        }).collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(n).map(|(ex, _)| ex).collect()
    }
}

pub fn format_prompt(task: &str, examples: &[&Example]) -> String {
    let mut prompt = String::new();
    for (i, ex) in examples.iter().enumerate() {
        prompt.push_str(&format!(
            "Example {}:\nInput: {}\nOutput: {}\n\n",
            i + 1, ex.input, ex.output
        ));
    }
    prompt.push_str(&format!("Now your task:\n{}", task));
    prompt
}
