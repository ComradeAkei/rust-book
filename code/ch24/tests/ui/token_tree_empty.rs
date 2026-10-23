// Паника процедурного макроса: ошибка без хорошего спана.
const TREE: &str = ch24::token_tree!();

fn main() {
    println!("{TREE}");
}
