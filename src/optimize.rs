use std::f32::INFINITY;

/// Smaller score is better
/// TODO: also try accepting worse scores on high temperatures
/// (requires kinda normalized scores); maybe as a new function
pub fn optimize<T: PartialEq>(
    mut value: T,
    mutate: impl Fn(&T, f32) -> Option<T>,
    score: impl Fn(&T) -> f32,
    steps: i32,
) -> T {
    let mut old_score = INFINITY;
    for step in 0..steps {
        let temperature = (1. - step as f32 / steps as f32).powf(0.3);
        if let Some(new) = mutate(&value, temperature) {
            let new_score = score(&new);
            if new_score < old_score {
                old_score = new_score;
                value = new;
            }
        }
    }
    value
}
