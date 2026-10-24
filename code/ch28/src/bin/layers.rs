//! Раздел 28.1: строка в stdout через std, libc, rustix и nix, затем одна
//! и та же ошибка open на каждом уровне. Тест
//! `one_write_per_layer_and_two_ways_to_open` запускает программу под strace.

use ch28::layers::{open_errors, say_libc, say_nix, say_rustix, say_std};

fn main() -> std::io::Result<()> {
    say_std("std\n")?;
    say_libc("libc\n")?;
    say_rustix("rustix\n")?;
    say_nix("nix\n")?;
    for (layer, text, size) in open_errors("/нет/такого/файла") {
        say_std(&format!("{layer:6} {size} байт: {text}\n"))?;
    }
    Ok(())
}
