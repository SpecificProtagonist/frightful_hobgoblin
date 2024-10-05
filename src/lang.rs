use flate2::{write::GzEncoder, Compression};
use itertools::Itertools;
use nbt::{encode::write_compound_tag, CompoundTag, Tag};
use replay::invocation;
use std::{fs::File, io::Write, path::Path};

use crate::*;

// This should replace make_name

#[derive(Debug)]
pub struct Phonotactics {
    vowels: Vec<(f32, &'static str)>,
    consonants: Vec<(f32, &'static str)>,
    consonant_start: f32,
    diphthongs: Vec<(char, char)>,
    // TODO: reuse diphthongs
    double_vowels: f32,
    consonant_clusters: f32,
    double_consonants: HashMap<&'static str, f32>,
    word_length: f32,
    prefixes: Vec<(f32, String)>,
    suffixes: Vec<(f32, String)>,
}

impl Phonotactics {
    pub fn new() -> Self {
        let mut vowels = vec![(1., "a"), (1., "e"), (0.7, "i"), (0.7, "o"), (0.7, "u")];
        if rand(0.3) {
            for vowel in ["á", "é", "ó", "ú"] {
                if rand(0.5) {
                    vowels.push((0.2, vowel));
                }
            }
        } else if rand(0.15) {
            vowels.push((0.2, "ä"));
            vowels.push((0.2, "ö"));
            vowels.push((0.2, "ü"));
        }
        if rand(0.15) {
            vowels.remove(3);
            vowels.push((0.7, "y"));
        }
        if rand(0.08) {
            vowels.push((0.3, "æ"));
        }
        if rand(0.08) {
            vowels.push((0.3, "å"));
        }
        if rand(0.08) {
            vowels.push((0.4, "w"));
        }
        if rand(0.08) {
            // TODO: disallow doubling & start of word
            vowels.push((0.4, "r"));
        }

        let mut possible_consonants = vec![
            (1.0, "b"),
            (1.0, "c"),
            (1.0, "d"),
            (1.0, "f"),
            (1.0, "g"),
            (1.0, "h"),
            (1.0, "j"),
            (1.0, "k"),
            (1.0, "l"),
            (1.0, "m"),
            (1.0, "n"),
            (1.0, "p"),
            (1.0, "q"),
            (1.0, "r"),
            (1.0, "s"),
            (1.0, "t"),
            (0.5, "v"),
            (1.0, "w"),
            (0.2, "x"),
            (0.1, "y"),
            (0.4, "z"),
        ];
        if rand(0.3) {
            possible_consonants.push((1., "Þ"))
        }
        let softness = rand(-0.5..0.5);
        let mut consonants = vec![];
        for i in 0..rand(9..15) {
            let index = rand(0..possible_consonants.len());
            let (mut weight, consonant) = possible_consonants.remove(index);
            weight += rand(-0.8..0.);
            if ["k", "t", "p"].contains(&consonant) {
                weight -= softness
            }
            if ["g", "d", "b"].contains(&consonant) {
                weight += softness
            }
            weight /= i as f32 + 3.;
            consonants.push((weight.max(0.), consonant));
        }

        let consonant_start = rand(-0.3..1.4).powf(0.6).clamp(0., 1.);
        let double_vowels = rand(-0.8..0.4).clamp(0., 1.);
        let consonant_clusters = rand(-0.9..1.).clamp(0., 1.);
        let word_length = rand(1. ..8.);

        let do_double_consonants = rand(0.6);
        let double_consonants = consonants
            .iter()
            .map(|(_, c)| {
                (
                    *c,
                    if do_double_consonants {
                        rand(-1.5..1.).max(0.)
                    } else {
                        0.
                    },
                )
            })
            .collect();

        let mut diphthongs = Vec::new();
        if rand(0.5) {
            for pair in [
                ('a', 'e'),
                ('a', 'i'),
                ('i', 'e'),
                ('e', 'i'),
                ('e', 'u'),
                ('o', 'u'),
                ('o', 'i'),
            ] {
                if rand(0.3) {
                    diphthongs.push(pair);
                }
            }
        }

        let mut prefixes = Vec::new();
        if rand(0.15) {
            for _ in 0..3 {
                let mut syllable = String::new();
                if rand(0.4) {
                    syllable.push_str(rand_weighted(&consonants));
                }
                syllable.push_str(rand_weighted(&vowels));
                if rand(0.3) {
                    syllable.push_str(rand_weighted(&consonants));
                } else if rand(0.3) {
                    syllable.push_str(rand_weighted(&vowels));
                }
                prefixes.push((rand(0.02..0.08), syllable));
            }
        }

        let mut suffixes = Vec::new();
        if rand(if prefixes.is_empty() { 0.22 } else { 0.1 }) {
            for _ in 0..3 {
                let mut syllable = String::new();
                if rand(0.4) {
                    syllable.push_str(rand_weighted(&consonants));
                }
                syllable.push_str(rand_weighted(&vowels));
                if rand(0.3) {
                    syllable.push_str(rand_weighted(&consonants));
                } else if rand(0.3) {
                    syllable.push_str(rand_weighted(&vowels));
                }
                suffixes.push((rand(0.02..0.08), syllable));
            }
        }

        Self {
            vowels,
            consonants,
            consonant_start,
            diphthongs,
            double_vowels,
            consonant_clusters,
            double_consonants,
            word_length,
            prefixes,
            suffixes,
        }
    }

    pub fn word(&self) -> String {
        let mut word = String::new();
        for (chance, prefix) in &self.prefixes {
            if rand(*chance) {
                word.push_str(prefix);
                word.push('-');
                break;
            }
        }
        let mut prev_consonant = true;
        for i in 0..8 {
            if (i > 0) | rand(self.consonant_start) {
                let consonant = rand_weighted(&self.consonants);
                word.push_str(consonant);
                if rand(self.double_consonants[consonant]) & !prev_consonant {
                    word.push_str(consonant);
                }
            }
            let vowel = rand_weighted(&self.vowels);
            word.push_str(vowel);
            prev_consonant = false;
            if rand(self.double_vowels) & (i > 0) & (vowel != "æ") {
                word.push_str(match vowel {
                    "á" => "a",
                    "é" => "e",
                    "ó" => "o",
                    "ú" => "u",
                    _ => vowel,
                });
            } else {
                for (a, b) in &self.diphthongs {
                    if word.ends_with(*a) && rand(0.3) {
                        word.push(*b);
                    }
                }
            }
            if rand(self.consonant_clusters) {
                word.push_str(rand_weighted(&self.consonants));
                prev_consonant = true;
            }
            if rand(word.len() as f32 * rand(0. ..1.) / self.word_length) {
                break;
            }
        }
        for (chance, suffix) in &self.suffixes {
            if rand(*chance) {
                word.push('-');
                word.push_str(suffix);
                break;
            }
        }
        word
    }
}

impl Default for Phonotactics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Resource, Debug)]
pub struct Lang {
    pub phono: Phonotactics,
    words: Vec<(f32, String)>,
    comma_chance: f32,
}

impl Lang {
    pub fn new() -> Self {
        let phono = Phonotactics::new();
        let capitalize_words = rand(0.4);
        let mut words = (0..100).map(|_| phono.word()).collect_vec();
        words.sort_by_key(|word| word.len());
        let words = words
            .into_iter()
            .enumerate()
            .map(|(i, mut word)| {
                let chance = 1. + 1. / (i as f32 + 0.5);
                if capitalize_words && rand(0.2) {
                    word = uppercase(&word);
                }
                (chance, word)
            })
            .collect();
        let comma_chance = rand(-0.1..0.2).max(0.);
        Self {
            phono,
            words,
            comma_chance,
        }
    }

    pub fn sentence(&self) -> String {
        let mut sentence = String::new();
        for _ in 0..rand(1..9) {
            sentence.push_str(&rand_weighted(&self.words));
            if rand(self.comma_chance) {
                sentence.push(',');
            }
            sentence.push(' ');
        }
        sentence.push_str(&rand_weighted(&self.words));
        uppercase(&sentence) + rand_weighted(&[(1., "."), (0.5, "?"), (0.3, "!"), (0.2, "...")])
    }

    pub fn write_blurbs(&self, level_path: &Path) {
        let data_path = level_path.join("data/");
        std::fs::create_dir_all(&data_path).unwrap();
        let blurbs = Tag::List((0..200).map(|_| self.sentence().into()).collect());
        let mut nbt = CompoundTag::new();
        nbt.insert("DataVersion", DATA_VERSION);
        nbt.insert("data", {
            let mut nbt = CompoundTag::new();
            nbt.insert("contents", {
                let mut nbt = CompoundTag::new();
                nbt.insert("data", {
                    let mut data = CompoundTag::new();
                    data.insert("blurbs", blurbs);
                    data
                });
                nbt
            });
            nbt
        });
        let mut file = File::create(
            data_path.join(format!("command_storage_sim_{}_language.dat", invocation())),
        )
        .unwrap();
        let mut uncompressed = Vec::new();
        write_compound_tag(&mut uncompressed, &nbt).unwrap();
        GzEncoder::new(&mut file, Compression::new(1))
            .write_all(&uncompressed)
            .unwrap();
    }

    // TODO: name generation
}

impl Default for Lang {
    fn default() -> Self {
        Self::new()
    }
}

fn uppercase(str: &str) -> String {
    let boundary = str.chars().next().unwrap().len_utf8();
    str[0..boundary].to_uppercase() + &str[boundary..]
}
