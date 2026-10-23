// FR только читается, ICR только пишется: у их типов просто нет методов.
tasks_macros::include_svd!("../../../../code/ch24/svd/lm3s6965.svd", "UART0");

fn main() {
    // SAFETY: тест только компилируется.
    let uart = unsafe { uart0::steal() };
    uart.fr.write(0);
    let _ = uart.icr.read();
    uart.fr.modify(|v| v);
}
