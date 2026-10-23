// Аргумент — путь к static, а не выражение.
#[tasks_macros::timed(1 + 2)]
fn work() {}

fn main() {}
