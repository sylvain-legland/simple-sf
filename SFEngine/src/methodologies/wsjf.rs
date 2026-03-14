// Ref: FT-SSF-023

#[derive(Debug, Clone)]
pub struct WSJFItem {
    pub id: String,
    pub business_value: f64,
    pub time_criticality: f64,
    pub risk_reduction: f64,
    pub job_size: f64,
}

pub fn score(item: &WSJFItem) -> f64 {
    if item.job_size <= 0.0 {
        return 0.0;
    }
    (item.business_value + item.time_criticality + item.risk_reduction) / item.job_size
}

pub fn prioritize(items: &mut [WSJFItem]) {
    items.sort_by(|a, b| score(b).partial_cmp(&score(a)).unwrap_or(std::cmp::Ordering::Equal));
}

pub fn format_priority_list(items: &[WSJFItem]) -> String {
    let mut out = String::from("| # | ID | BV | TC | RR | Size | WSJF |\n");
    out.push_str("|---|----|----|----|----|------|------|\n");
    for (i, item) in items.iter().enumerate() {
        out.push_str(&format!(
            "| {} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.2} |\n",
            i + 1,
            item.id,
            item.business_value,
            item.time_criticality,
            item.risk_reduction,
            item.job_size,
            score(item),
        ));
    }
    out
}
