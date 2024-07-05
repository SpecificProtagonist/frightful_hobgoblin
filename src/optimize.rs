use bevy_math::FloatOrd;

/// Smaller score is better
// TODO: Handle failure better
pub fn optimize<T: PartialEq + Copy>(
    value: T,
    fun: impl Fn(&mut T, f32) -> f32,
    steps: i32,
    attempts: i32,
) -> Option<T> {
    // TODO: Maybe run this in parallel? (explicitly set rng at start of task)
    (0..attempts)
        .flat_map(move |_| {
            let mut value = value;
            let mut old_score = f32::INFINITY;
            for step in 0..steps {
                let temperature = (1. - step as f32 / steps as f32).powf(0.3);
                let mut new = value;
                let new_score = fun(&mut new, temperature);
                if new_score < old_score {
                    old_score = new_score;
                    value = new;
                }
            }
            (old_score < f32::INFINITY).then_some((old_score, value))
        })
        .min_by_key(|(score, _)| FloatOrd(*score))
        .map(|(_, value)| value)
}
