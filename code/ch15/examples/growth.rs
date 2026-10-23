//! Разделы 15.1 и 15.7: как растёт `Vec<u8>` и что при этом делает `malloc` из
//! glibc. Пример печатает каждую новую ёмкость и переехал ли буфер; под
//! strace видно, какой системный вызов стоит за каждым ростом.
//!
//! ```console
//! $ cargo build --release -p ch15 --example growth
//! $ strace -e trace=brk,mmap,mremap,munmap,write target/release/examples/growth
//! $ strace -e trace=brk,mmap,mremap,munmap,write target/release/examples/growth twice
//! ```
//!
//! `twice` растит вектор до 4 МиБ дважды: после освобождения первого
//! большого блока glibc поднимает порог mmap, и второй рост идёт в куче
//! через brk.
use std::io::Write;

// ANCHOR: grow
fn grow(n: usize, out: &mut impl Write) {
    let mut v: Vec<u8> = Vec::new();
    let (mut cap, mut ptr, mut moves) = (0, v.as_ptr(), 0);
    for i in 0..n {
        v.push(i as u8);
        if v.capacity() != cap {
            let moved = v.as_ptr() != ptr;
            moves += moved as usize;
            let how = if moved { "новый адрес" } else { "на месте" };
            // Строка попадает в strace как write и отмечает рост.
            writeln!(out, "ёмкость {:8}: {how}", v.capacity()).unwrap();
            (cap, ptr) = (v.capacity(), v.as_ptr());
        }
    }
    writeln!(out, "переездов: {moves}").unwrap();
}
// ANCHOR_END: grow

fn main() {
    let mut out = std::io::stdout().lock();
    let twice = std::env::args().nth(1).as_deref() == Some("twice");
    grow(4 << 20, &mut out);
    if twice {
        writeln!(out, "--- второй раз").unwrap();
        grow(4 << 20, &mut out);
    }
}
