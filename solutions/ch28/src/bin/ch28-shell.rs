//! Задача 28.C: мини-оболочка. `cargo run -p ch28-solutions --bin
//! ch28-shell` в терминале — интерактивный режим с управлением
//! заданиями; со stdin из канала или файла — чтение команд построчно.

fn main() {
    std::process::exit(ch28_solutions::c_shell::repl());
}
