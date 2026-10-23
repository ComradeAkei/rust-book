//! Задача A: раскрытие `#[derive(Debug)]` и `#[entry]` из прошивки
//! `firmware/ch24-expand` (rustc 1.97, cortex-m-rt 0.7.7). Тест сравнивает
//! без учёта пробелов и переводов строк.

// ANCHOR: predicted
pub fn predicted() -> [&'static str; 4] {
    [
        // 1. #[derive(Debug)] на struct Point { x: i32, y: i32 }
        r#"#[automatically_derived]
        impl ::core::fmt::Debug for Point {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field2_finish(f, "Point", "x",
                    &self.x, "y", &&self.y)
            }
        }"#,
        // 2. #[derive(Debug)] на enum Mode { Off, Blink(u8) }
        r#"#[automatically_derived]
        impl ::core::fmt::Debug for Mode {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    Mode::Off => ::core::fmt::Formatter::write_str(f, "Off"),
                    Mode::Blink(__self_0) =>
                        ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Blink",
                            &__self_0),
                }
            }
        }"#,
        // 3. #[entry]: переходник под символом main.
        r#"#[doc(hidden)]
        #[export_name = "main"]
        pub unsafe extern "C" fn __cortex_m_rt_main_trampoline() {
            #[allow(static_mut_refs)]
            __cortex_m_rt_main({
                    static mut BOOTS: u32 = 0;
                    unsafe { &mut BOOTS }
                })
        }"#,
        // 4. #[entry]: функция пользователя, переименованная, со static mut
        //    в аргументе.
        r#"fn __cortex_m_rt_main(#[allow(non_snake_case)] BOOTS: &'static mut u32) -> ! {
            *BOOTS += 1;"#,
    ]
}
// ANCHOR_END: predicted
