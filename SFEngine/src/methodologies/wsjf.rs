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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_calculation() {
        let item = WSJFItem { id: "a".into(), business_value: 8.0, time_criticality: 5.0, risk_reduction: 2.0, job_size: 3.0 };
        assert!((score(&item) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn score_zero_size_returns_zero() {
        let item = WSJFItem { id: "z".into(), business_value: 10.0, time_criticality: 5.0, risk_reduction: 5.0, job_size: 0.0 };
        assert_eq!(score(&item), 0.0);
    }

    #[test]
    fn prioritize_sorts_descending() {
        let mut items = vec![
            WSJFItem { id: "low".into(), business_value: 1.0, time_criticality: 1.0, risk_reduction: 1.0, job_size: 10.0 },
            WSJFItem { id: "high".into(), business_value: 10.0, time_criticality: 5.0, risk_reduction: 5.0, job_size: 2.0 },
        ];
        prioritize(&mut items);
        assert_eq!(items[0].id, "high");
        assert_eq!(items[1].id, "low");
    }
}
