//! Раздел 19.2: `global_asm!` и naked-функции. `global_asm!` вставляет
//! ассемблер на уровне модуля — целую функцию или данные. naked-функция
//! (`#[unsafe(naked)]` + `naked_asm!`) — это функция вообще без пролога и
//! эпилога, которые обычно генерирует компилятор: её тело — ровно ваши
//! инструкции. Это нужно там, где важен точный кадр стека: обработчики
//! прерываний и переключение контекста.

use core::arch::{global_asm, naked_asm};

// ANCHOR: global
// Целая функция на ассемблере через global_asm!: ch19_triple(x) = x*3,
// одной инструкцией lea (x + x*2). Объявляем её как внешнюю и зовём.
global_asm!(".globl ch19_triple", "ch19_triple:", "    lea rax, [rdi + rdi*2]", "    ret",);

unsafe extern "C" {
    fn ch19_triple(x: u64) -> u64;
}

/// Безопасная обёртка над функцией из `global_asm!`.
pub fn triple(x: u64) -> u64 {
    // SAFETY: ch19_triple — чистая арифметика над аргументом в rdi, результат
    // в rax по C-ABI; памяти и стека не трогает.
    unsafe { ch19_triple(x) }
}
// ANCHOR_END: global

// ANCHOR: switch
/// Сохранённый контекст сопрограммы — только указатель стека: всё остальное
/// (callee-saved регистры) лежит на этом стеке.
#[repr(C)]
pub struct Context {
    pub rsp: u64,
}

/// Переключить контекст: сохранить регистры текущей сопрограммы на её стек,
/// переключить `rsp` на стек следующей и восстановить её регистры. Функция
/// naked — ни пролога, ни эпилога: кадр стека мы раскладываем сами, поэтому
/// после `ret` исполнение идёт с того места, где другая сопрограмма ушла в
/// `switch` (или со стартового адреса, если стек подготовлен вручную).
///
/// # Safety
///
/// `from` и `to` указывают на валидные `Context`. `to.rsp` либо сохранён
/// прошлым вызовом `switch`, либо указывает на корректно собранный стартовый
/// кадр (шесть callee-saved значений и адрес возврата сверху). Соглашение о
/// вызовах — `extern "C"`: `from` в rdi, `to` в rsi.
#[unsafe(naked)]
pub unsafe extern "C" fn switch(from: *mut Context, to: *const Context) {
    naked_asm!(
        // Сохранить callee-saved регистры текущей сопрограммы на её стек.
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        // from->rsp = rsp
        "mov [rdi], rsp",
        // rsp = to->rsp — переключились на чужой стек.
        "mov rsp, [rsi]",
        // Восстановить его регистры в обратном порядке.
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        // ret снимает со стека адрес возврата той сопрограммы.
        "ret",
    );
}
// ANCHOR_END: switch

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_asm_triple() {
        assert_eq!(triple(7), 21);
        assert_eq!(triple(0), 0);
    }

    // Один прогон переключения: из main в сопрограмму и обратно.
    static mut MAIN: Context = Context { rsp: 0 };
    static mut CO: Context = Context { rsp: 0 };
    static mut STACK: [u64; 512] = [0; 512];
    static mut HITS: u32 = 0;

    extern "C" fn coroutine() -> ! {
        // SAFETY: единственный доступ к HITS/CO — здесь и в тесте, строго по
        // очереди (кооперативно), гонки нет.
        unsafe {
            // Через сырой указатель, не ссылку на static mut.
            let hits = &raw mut HITS;
            *hits += 1;
            // Вернуть управление main.
            switch(&raw mut CO, &raw const MAIN);
        }
        unreachable!("после возврата в main сюда не приходят")
    }

    #[test]
    fn switch_runs_coroutine_then_returns() {
        // SAFETY: собираем стартовый кадр сопрограммы: шесть callee-saved
        // значений (любых) и адрес возврата = coroutine сверху. Первый switch
        // «восстановит» этот кадр и ret-нется в coroutine.
        unsafe {
            let base = (&raw mut STACK).cast::<u64>();
            let sp = base.add(512).sub(7);
            *sp.add(6) = coroutine as *const () as u64; // адрес для ret
            (&raw mut CO).write(Context { rsp: sp as u64 });

            switch(&raw mut MAIN, &raw const CO);
            assert_eq!((&raw const HITS).read(), 1);
        }
    }
}
