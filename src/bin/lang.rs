use frightful_hobgoblin::lang::Lang;

fn main() {
    let lang = Lang::new();
    for _ in 0..15 {
        print!("{} ", lang.sentence());
    }
}
