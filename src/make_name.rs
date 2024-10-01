use itertools::Itertools;

use crate::*;

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
        name.extend(uppercase(prefixes.choose()));
        name += middle.choose();
        if name.ends_with("thorp") {
            name += "e"
        }
    } else if rand(0.3) {
        name.extend(uppercase(prefixes.choose()));
        name += suffixes.choose();
    } else if rand(0.5) {
        name.extend(uppercase(middle.choose()));
        name += suffixes.choose();
    } else {
        name.extend(uppercase(prefixes.choose()));
        name += middle.choose();
        name += suffixes.choose();
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
        name.extend(uppercase(standalone.choose()));
    }

    name
}

fn uppercase(word: &'static str) -> impl Iterator<Item = char> {
    let mut iter = word.chars();
    iter.next()
        .map(char::to_uppercase)
        .into_iter()
        .flatten()
        .chain(iter)
}

pub fn tavern_name() -> String {
    let animal = [
        "Hare", "Stag", "Hart", "Buck", "Cow", "Heifer", "Calf", "Bull", "Birds", "Pheasant",
        "Cock", "Rooster", "Dove", "Crane", "Swan", "Tit", "Ostrich", "Horse", "Stallion", "Dog",
        "Bitch", "Dolphin", "Lion", "Ram", "Bear", "Dragon", "Rabbit", "Goat",
    ];
    let person_trait = [
        "Old", "Young", "Jovial", "Lazy", "Tipsy", "Drunk", "Drunken", "Sleeping", "Bold",
        "Cunning", "Fat",
    ];
    let occupation = [
        "Porter",
        "Blacksmith",
        "Carpenter",
        "Weaver",
        "Fisherman",
        "Spinner",
        "Cobbler",
        "Cordwainer",
        "Ratcatcher",
        "Glazier",
        "Potter",
        "Sergeant",
        "Pilgrim",
        "Bishop",
    ];
    let number = [(2., "Three"), (0.3, "Double"), (0.3, "Two"), (0.3, "Dozen")];
    let arms = [
        "Castle", "Cross", "Shield", "Crown", "Antler", "Band", "Leaf", "Arrow", "Head", "Cup",
        "Hammer", "Flute", "Drum", "Sickle", "Hatchet", "Plow", "Rose",
    ];
    let thingy = arms
        .iter()
        .map(|&s| (2.5, s))
        .chain(animal.iter().map(|&s| (1., s)))
        .collect_vec();
    let tavern = [
        (1.5, "Tavern"),
        (1.1, "Inn"),
        (0.6, "Lodge"),
        (0.5, "House"),
        (0.5, "Auberge"),
        (0.4, "Bethel"),
        (0.4, "Pub"),
        (0.3, "Rest"),
        (0.3, "Respite"),
    ];
    let mut name = rand_weighted(&[
        (1., format!("{}'s Arms", occupation.choose())),
        (
            1.,
            format!("{} {}", person_trait.choose(), occupation.choose()),
        ),
        (1., format!("{} {}s", rand_weighted(&number), arms.choose())),
        (
            2.,
            format!("{} and {}", rand_weighted(&thingy), rand_weighted(&thingy)),
        ),
        (
            1.4,
            format!("{} {}", person_trait.choose(), animal.choose()),
        ),
    ]);
    if rand(0.4) {
        name.insert_str(0, "The ");
    }
    if rand(0.3) {
        name = format!("{name} {}", rand_weighted(&tavern));
    }
    name
}

pub fn make_tokipona_name() -> String {
    loop {
        let consonants = ['m', 'n', 'p', 't', 'k', 's', 'w', 'l', 'j'];
        let vowels = [(35., 'a'), (25., 'i'), (15., 'e'), (15., 'o'), (10., 'u')];

        let mut name = "jan ".to_owned();

        if rand(0.7) {
            name.push(*consonants.choose());
        }
        name.push(rand_weighted(&vowels));
        if rand(0.1) {
            name.push('n');
        }
        for _ in 0..rand_weighted(&[(2., 1), (7., 2), (1., 3)]) {
            name.push(*consonants.choose());
            name.push(rand_weighted(&vowels));
            if rand(0.1) {
                name.push('n');
            }
        }

        if !(name.contains("ti")
            | name.contains("wo")
            | name.contains("wu")
            | name.contains("ji")
            | name.contains("nn")
            | name.contains("nm"))
        {
            return name;
        }
    }
}
