/// Computes the reputation score based on reports/chats in the last 30 days.
/// Score is clamped to [0.0, 1.0].
pub fn calculate_reputation(
    unique_reporters_30d: f64,
    total_chats_30d: f64,
    penalty_weight: f64,
) -> f64 {
    // Validate inputs are non-negative and finite
    if total_chats_30d <= 0.0
        || !total_chats_30d.is_finite()
        || !unique_reporters_30d.is_finite()
        || !penalty_weight.is_finite()
        || unique_reporters_30d < 0.0
        || penalty_weight < 0.0
    {
        return 1.0;
    }

    let raw = 1.0 - (unique_reporters_30d / total_chats_30d) * penalty_weight;

    if raw.is_nan() || raw.is_infinite() {
        return 1.0; // default to full reputation
    }
    raw.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_reputation_with_no_reports() {
        assert_eq!(calculate_reputation(0.0, 10.0, 1.0), 1.0);
    }

    #[test]
    fn half_reported_with_weight_one() {
        // 5 reporters out of 10 chats, weight 1.0 => 1 - 0.5 = 0.5
        assert!((calculate_reputation(5.0, 10.0, 1.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn clamped_to_zero_when_fully_reported() {
        // 10 reporters out of 10 chats, weight 2.0 => 1 - 2.0 = -1.0, clamped to 0.0
        assert_eq!(calculate_reputation(10.0, 10.0, 2.0), 0.0);
    }

    #[test]
    fn clamped_to_one_when_negative_penalty() {
        // Even though the result would be > 1.0, it should be clamped
        assert_eq!(calculate_reputation(0.0, 100.0, 1.0), 1.0);
    }

    #[test]
    fn zero_chats_returns_default() {
        assert_eq!(calculate_reputation(5.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn negative_chats_returns_default() {
        assert_eq!(calculate_reputation(5.0, -1.0, 1.0), 1.0);
    }

    #[test]
    fn nan_reporters_returns_default() {
        assert_eq!(calculate_reputation(f64::NAN, 10.0, 1.0), 1.0);
    }

    #[test]
    fn nan_chats_returns_default() {
        assert_eq!(calculate_reputation(5.0, f64::NAN, 1.0), 1.0);
    }

    #[test]
    fn nan_weight_returns_default() {
        assert_eq!(calculate_reputation(5.0, 10.0, f64::NAN), 1.0);
    }

    #[test]
    fn infinity_reporters_returns_default() {
        assert_eq!(calculate_reputation(f64::INFINITY, 10.0, 1.0), 1.0);
    }

    #[test]
    fn infinity_chats_returns_default() {
        assert_eq!(calculate_reputation(5.0, f64::INFINITY, 1.0), 1.0);
    }

    #[test]
    fn infinity_weight_returns_default() {
        assert_eq!(calculate_reputation(5.0, 10.0, f64::INFINITY), 1.0);
    }

    #[test]
    fn negative_reporters_returns_default() {
        assert_eq!(calculate_reputation(-1.0, 10.0, 1.0), 1.0);
    }

    #[test]
    fn negative_weight_returns_default() {
        assert_eq!(calculate_reputation(5.0, 10.0, -1.0), 1.0);
    }

    #[test]
    fn result_clamped_between_zero_and_one() {
        for reporters in [0.0, 1.0, 5.0, 10.0, 100.0] {
            for chats in [1.0, 5.0, 10.0, 100.0] {
                for weight in [0.1, 0.5, 1.0, 2.0, 10.0] {
                    let score = calculate_reputation(reporters, chats, weight);
                    assert!(
                        (0.0..=1.0).contains(&score),
                        "score {score} out of range for reporters={reporters}, chats={chats}, weight={weight}"
                    );
                }
            }
        }
    }
}
