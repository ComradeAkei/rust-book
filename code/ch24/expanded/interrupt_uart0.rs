#[unsafe(export_name = "UART0")]
pub unsafe extern "C" fn __ch24_UART0_trampoline() {
    __ch24_UART0({
            static mut COUNT: u32 = 0;
            unsafe { &mut *&raw mut COUNT }
        })
}
#[allow(non_snake_case)]
fn __ch24_UART0(#[allow(non_snake_case)] COUNT: &mut u32) {
    let _ = interrupt::UART0;
    *COUNT += 1;
    UART0_CALLS.store(*COUNT, Ordering::Relaxed);
}
