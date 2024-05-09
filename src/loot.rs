use crate::*;
use itertools::Itertools;

pub fn chest() -> String {
    let tools: (&[_], _) = (
        &[
            (1., ("iron_pickaxe", 1..=1)),
            (1., ("iron_shovel", 1..=1)),
            (1., ("iron_axe", 1..=1)),
            (1., ("flint_and_steel", 1..=1)),
        ],
        1..=3,
    );
    let food: (&[_], _) = (&[(1., ("bread", 1..=13)), (0.3, ("sugar", 1..=5))], 3..=7);
    let tables = &[(1., tools), (1., food)];

    let mut items = Vec::new();
    let (table, roll_count) = rand_weighted(tables);
    for _ in 0..rand_range(roll_count) {
        let (item, count) = rand_weighted(table);
        items.push((item, rand_range(count)));
    }

    snbt(&items)
}

fn snbt(items: &[(&str, i32)]) -> String {
    let mut out = "Items:[".to_owned();
    let mut slots = (0..27).collect_vec();
    for (i, (item, count)) in items.iter().enumerate() {
        if i != 0 {
            out.push(',');
        }
        let slot = slots.remove(rand_range(0..slots.len()));
        out.push_str(&format!("{{Slot:{slot}b,Count:{count},id:\"{item}\"}}"));
    }
    out.push(']');
    out
}