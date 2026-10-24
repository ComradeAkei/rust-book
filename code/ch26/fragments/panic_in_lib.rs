// Раздел 26.6: обработчик паники в библиотеке ломает её тесты на хосте.
// ANCHOR: panic_in_lib
#![no_std]

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn adds() {
        assert_eq!(super::add(2, 2), 4);
    }
}
// ANCHOR_END: panic_in_lib
