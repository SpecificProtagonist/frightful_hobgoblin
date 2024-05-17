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
    let mut slots = (0..27).collect_vec();
    let (table, roll_count) = rand_weighted(tables);
    for _ in 0..rand_range(roll_count) {
        let (item, count) = rand_weighted(table);
        let slot = slots.remove(rand_range(0..slots.len()));
        items.push((item, rand_range(count), slot));
    }

    snbt(&items)
}

pub fn smoker() -> String {
    let wood = format!("{}_planks", center_biome().random_tree_species().to_str());
    let fuel = rand_weighted(&[(1., "coal"), (1., &wood), (0.6, "stick")]);
    // TODO use biomes on level to decide food
    // (e.g. fish depends on ocean/river; salmon prefers cold water)
    let output = rand_weighted(&[
        (1., "cooked_cod"),
        (1., "cooked_salmon"),
        (2., "baked_potato"),
        (1.4, "cooked_mutton"),
        (0.4, "cooked_rabbit"),
        (0.4, "cooked_chicken"),
    ]);
    snbt(&[(fuel, rand_range(0..8), 1), (output, rand_range(0..7), 2)])
}

fn snbt(items: &[(&str, i32, i32)]) -> String {
    let mut out = "Items:[".to_owned();
    for (i, (item, count, slot)) in items.iter().enumerate() {
        if i != 0 {
            out.push(',');
        }
        out.push_str(&format!("{{Slot:{slot}b,Count:{count},id:\"{item}\"}}"));
    }
    out.push(']');
    out
}
