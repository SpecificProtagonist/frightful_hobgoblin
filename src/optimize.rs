use std::f32::INFINITY;

/// Smaller score is better
/// TODO: also try accepting worse scores on high temperatures
/// (requires kinda normalized scores); maybe as a new function
pub fn optimize<T: PartialEq + Clone>(
    mut value: T,
    fun: impl Fn(T, f32) -> Option<(T, f32)>,
    steps: i32,
) -> Option<T> {
    let mut old_score = INFINITY;
    let mut success = false;
    for step in 0..steps {
        let temperature = (1. - step as f32 / steps as f32).powf(0.3);
        if let Some((new, new_score)) = fun(value.clone(), temperature) {
            success = true;
            if new_score < old_score {
                old_score = new_score;
                value = new;
            }
        }
    }
    success.then_some(value)
}
