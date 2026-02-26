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
