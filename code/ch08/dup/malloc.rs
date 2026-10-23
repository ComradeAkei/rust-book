// Раздел 8.7: имя без манглинга заменяет одноимённую функцию libc во
// всём процессе. Поэтому атрибут no_mangle в edition 2024 небезопасен.
// ANCHOR: malloc
#[unsafe(no_mangle)]
pub extern "C" fn malloc(_size: usize) -> *mut u8 {
    core::ptr::null_mut()
}

fn main() {
    let v = vec![0u8; 16];
    println!("{}", v.len());
}
// ANCHOR_END: malloc
