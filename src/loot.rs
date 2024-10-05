use crate::*;
use itertools::Itertools;

pub fn chest() -> String {
    let tools: (&[_], _) = (
        &[
            (1., ("iron_hoe", 1..=1)),
            (1., ("iron_pickaxe", 1..=1)),
            (1., ("iron_shovel", 1..=1)),
            (1., ("iron_axe", 1..=1)),
            (1., ("flint_and_steel", 1..=1)),
            (1., ("shears", 1..=1)),
        ],
        1..=3,
    );
    let food: (&[_], _) = (
        &[
            (1., ("bread", 1..=13)),
            (0.3, ("sugar", 1..=5)),
            (0.3, ("egg", 1..=8)),
        ],
        3..=7,
    );
    let clothes: (&[_], _) = (
        &[
            (1., ("leather_boots", 1..=1)),
            (1., ("leather_leggings", 1..=1)),
            (1., ("leather_chestplate", 1..=1)),
            (1., ("leather_helmet", 1..=1)),
            (0.8, ("chainmail_chestplate", 1..=1)),
            (0.4, ("turtle_helmet", 1..=1)),
        ],
        3..=4,
    );
    let dyes: (&[_], _) = (
        &[
            (1., ("white", 1..=4)),
            (1., ("orange", 1..=4)),
            (1., ("magenta", 1..=4)),
            (1., ("light_blue", 1..=4)),
            (1., ("yellow", 1..=4)),
            (1., ("lime", 1..=4)),
            (1., ("pink", 1..=4)),
            (1., ("gray", 1..=4)),
            (1., ("light_gray", 1..=4)),
            (1., ("cyan", 1..=4)),
            (1., ("purple", 1..=4)),
            (1., ("blue", 1..=4)),
            (1., ("brown", 1..=4)),
            (1., ("green", 1..=4)),
            (1., ("red", 1..=4)),
            (1., ("black", 1..=4)),
            (3., ("feather", 1..=1)),
        ],
        4..=7,
    );
    let tables = &[(1., tools), (1., clothes), (1.5, food), (0.4, dyes)];

    let mut items = Vec::new();
    let mut slots = (0..27).collect_vec();
    let (table, roll_count) = rand_weighted(tables);
    for _ in 0..rand(roll_count) {
        let (item, count) = rand_weighted(table);
        let slot = slots.remove(rand(0..slots.len()));
        items.push((item, rand(count), slot));
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
    snbt(&[(fuel, rand(0..8), 1), (output, rand(0..7), 2)])
}

fn snbt(items: &[(&str, i32, i32)]) -> String {
    let mut out = "Items:[".to_owned();
    for (i, (item, count, slot)) in items.iter().enumerate() {
        if i != 0 {
            out.push(',');
        }
        out.push_str(&format!("{{Slot:{slot}b,count:{count},id:\"{item}\"}}"));
    }
    out.push(']');
    out
}

fn potion() -> String {
    // TODO: Components
    format!(
        "potion,tag:{{Potion:\"{}\"}}",
        [
            "mundane",
            "thick",
            "awkward",
            "strong_regeneration",
            "long_fire_resistance",
            "long_swiftness",
            "water_breathing",
            "long_night_vision",
            "invisibility"
        ]
        .choose()
    )
}

pub fn brewing_stand() -> String {
    let mut items = vec![("blaze_powder", rand(0..10), 4)];
    let potion = potion();
    for i in 0..3 {
        if rand(0.8) {
            items.push((&potion, 1, i))
        }
    }
    snbt(&items)
}
