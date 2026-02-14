/// Computes the reputation score based on reports/chats in the last 30 days.
/// Score is clamped to [0.0, 1.0].
pub fn calculate_reputation(
    unique_reporters_30d: f64,
    total_chats_30d: f64,
    penalty_weight: f64,
) -> f64 {
    if total_chats_30d <= 0.0 {
        return 1.0;
    }

    let raw = 1.0 - (unique_reporters_30d / total_chats_30d) * penalty_weight;
    raw.clamp(0.0, 1.0)
}
