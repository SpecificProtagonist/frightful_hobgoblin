use rand::Rng;

use crate::*;

// Todo: Villager names
// Todo: make toponyms take features into account (not needed for other villages mentioned but not generated)
//       that's also something to be mentioned in the village chronicle
// Todo: different generators for different biomes

pub fn make_town_name() -> String {
    let prefixes = &[
        "aber", "ard", "ash", "ast", "auch", "bre", "car", "dal", "inch", "kil", "lang", "nor",
        "rother", "shep", "stan", "sut",
    ];
    let middle = &[
        "ac", "avon", "beck", "fos", "garth", "grim", "holm", "hamp", "kirk", "mere", "thorp",
        "pit",
    ];
    let suffixes = &[
        "berry", "bourne", "burry", "bourgh", "borough", "by", "carden", "cester", "combe", "cott",
        "dale", "esk", "ey", "field", "fold", "ham", "hurst", "ing", "more", "ness", "rig", "pool",
        "stead", "ter", "ton", "wich", "wick", "worth",
    ];
    let standalone = &["ben", "eglos", "hayes", "law", "minster", "shaw", "stoke"];

    // Todo: Experiment with probabilities
    let mut name = String::new();
    if rand(0.25) {
        name.extend(uppercase(select(prefixes)));
        name += select(middle);
        if name.ends_with("thorp") {
            name += "e"
        }
    } else if rand(0.3) {
        name.extend(uppercase(select(prefixes)));
        name += select(suffixes);
    } else if rand(0.5) {
        name.extend(uppercase(select(middle)));
        name += select(suffixes);
    } else {
        name.extend(uppercase(select(prefixes)));
        name += select(middle);
        name += select(suffixes);
    }

    name = name
        .replace("pb", "b")
        .replace("hh", "h")
        .replace("tt", "t");

    if rand(0.25) {
        if rand(0.3) {
            name += "-le-"
        } else {
            name += " ";
        }
        name.extend(uppercase(select(standalone)));
    }

    name
}

fn select<'a>(list: &'a [&'a str]) -> &'a str {
    list[rand_range(0, list.len() as i32) as usize]
}

fn uppercase(word: &'static str) -> impl Iterator<Item = char> {
    let mut iter = word.chars();
    iter.next()
        .map(char::to_uppercase)
        .into_iter()
        .flatten()
        .chain(iter)
}
