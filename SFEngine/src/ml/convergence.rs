// Ref: FT-SSF-022
// Convergence Detection — identify score trends and stopping conditions

#[derive(Debug, PartialEq)]
pub enum Trend {
    Converging,
    Diverging,
    Oscillating,
    Plateau,
    Insufficient,
}

pub fn detect_trend(scores: &[f64]) -> Trend {
    if scores.len() < 3 {
        return Trend::Insufficient;
    }

    let tail = &scores[scores.len().saturating_sub(3)..];

    // Plateau: std_dev < 0.05 for last 3 points
    let mean = tail.iter().sum::<f64>() / tail.len() as f64;
    let variance = tail.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / tail.len() as f64;
    let std_dev = variance.sqrt();
    if std_dev < 0.05 {
        return Trend::Plateau;
    }

    let diffs: Vec<f64> = scores.windows(2).map(|w| w[1] - w[0]).collect();

    // Converging: monotonically increasing or differences are decreasing (approaching target)
    let all_positive = diffs.iter().all(|d| *d >= 0.0);
    if all_positive {
        return Trend::Converging;
    }

    let abs_diffs: Vec<f64> = diffs.iter().map(|d| d.abs()).collect();
    let diffs_decreasing = abs_diffs.windows(2).all(|w| w[1] <= w[0]);
    if diffs_decreasing && diffs.last().map(|d| *d >= 0.0).unwrap_or(false) {
        return Trend::Converging;
    }

    // Diverging: monotonically decreasing
    if diffs.iter().all(|d| *d <= 0.0) {
        return Trend::Diverging;
    }

    // Oscillating: alternating up/down
    let alternating = diffs.windows(2).all(|w| w[0].signum() != w[1].signum());
    if alternating && diffs.len() >= 2 {
        return Trend::Oscillating;
    }

    Trend::Converging
}

pub fn should_stop(scores: &[f64], threshold: f64) -> bool {
    let trend = detect_trend(scores);
    match trend {
        Trend::Plateau => scores.last().map(|s| *s >= threshold).unwrap_or(false),
        Trend::Converging => scores.last().map(|s| *s >= threshold).unwrap_or(false),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_converging() {
        assert_eq!(detect_trend(&[1.0, 2.0, 3.0]), Trend::Converging);
    }

    #[test]
    fn detect_plateau() {
        assert_eq!(detect_trend(&[5.0, 5.0, 5.0]), Trend::Plateau);
    }

    #[test]
    fn detect_oscillating() {
        assert_eq!(detect_trend(&[1.0, 3.0, 1.0]), Trend::Oscillating);
    }

    #[test]
    fn detect_diverging() {
        assert_eq!(detect_trend(&[3.0, 2.0, 1.0]), Trend::Diverging);
    }

    #[test]
    fn detect_insufficient() {
        assert_eq!(detect_trend(&[1.0]), Trend::Insufficient);
    }
}
