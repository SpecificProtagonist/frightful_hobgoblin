use crate::geometry::rand;

// Todo: Villager names
// Todo: make toponyms take features into account
//       that's also something to be mentioned in the village chronicle
// Todo: prevent bad phonome combinations
// Todo: different generators for different biomes

pub fn make_town_name() -> String {
    let prefixes = &[
        "aber", "ard", "ash", "ast", "auch", "bre", "dal", "kil", "lang", "nor", "rother", "shep",
        "stan", "sut",
    ];
    let middle = &[
        "ac", "avon", "beck", "fos", "garth", "holm", "hamp", "mere", "thorpe",
    ];
    let suffixes = &[
        "berry", "bourne", "burry", "bourgh", "borough", "by", "carden", "combe", "cott", "dale",
        "esk", "ey", "field", "ham", "hurst", "ing", "stead", "ter", "ton", "wich", "wick",
        "worth",
    ];
    let standalone = &["ben", "eglos", "hayes", "law", "minster", "shaw", "stoke"];

    // Todo: Experiment with probabilities
    let mut name = String::new();
    if rand(0.27) {
        name.extend(uppercase(select(prefixes)));
        name += select(middle);
    } else if rand(0.4) {
        name.extend(uppercase(select(middle)));
        name += select(suffixes);
    } else {
        name.extend(uppercase(select(prefixes)));
        name += select(suffixes);
    }

    if rand(0.3) {
        name += " ";
        name.extend(uppercase(select(standalone)));
    }

    name
}

fn select<'a>(list: &'a [&'a str]) -> &'a str {
    use rand::Rng;
    list[rand::thread_rng().gen_range(0, list.len())]
}

fn uppercase(word: &'static str) -> impl Iterator<Item = char> {
    let mut iter = word.chars();
    iter.next()
        .map(char::to_uppercase)
        .into_iter()
        .flatten()
        .chain(iter)
}
