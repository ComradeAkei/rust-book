//! Раздел A.4: `std::process::exit` завершает процесс сразу и не вызывает
//! деструкторы, поэтому буфер `BufWriter` до вывода не доходит. В C
//! функция `exit` сбрасывает буферы stdio (см. `c/exit_flush.c`).

use std::io::{BufWriter, Write};

// ANCHOR: exit
fn main() {
    let mut out = BufWriter::new(std::io::stdout());
    writeln!(out, "строка в буфере BufWriter").unwrap();
    std::process::exit(0); // деструктор `out` не выполнится
}
// ANCHOR_END: exit
