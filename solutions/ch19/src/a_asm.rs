//! Задача A: три примитива через `asm!`. Каждый показывает свою тонкость
//! операндов, которой не было в примерах раздела 19.1.

// ANCHOR: solution
/// Циклический сдвиг влево. `rol` на x86 берёт счётчик сдвига ТОЛЬКО из
/// регистра `cl` (или как непосредственную константу) — отсюда привязка
/// операнда к фиксированному регистру `in("cl")`, а не к произвольному `reg`.
pub fn rotate_left(x: u64, n: u32) -> u64 {
    let out;
    // SAFETY: rol — чистая арифметика над регистром; счётчик в cl берётся по
    // модулю 64 (как и у `u64::rotate_left`). Памяти и стека нет, флаги не
    // важны вызывающему — pure/nomem/nostack честны.
    unsafe {
        core::arch::asm!(
            "rol {x}, cl",
            x = inout(reg) x => out,
            in("cl") n as u8,
            options(pure, nomem, nostack),
        );
    }
    out
}

/// Число ведущих нулей. `bsr` находит индекс старшего установленного бита, но
/// при нулевом входе результат НЕ ОПРЕДЕЛЁН, поэтому ноль обрабатываем до
/// ассемблера. Для `x != 0`: ведущих нулей = 63 − индекс старшего бита.
pub fn leading_zeros(x: u64) -> u32 {
    if x == 0 {
        return 64;
    }
    let idx: u64;
    // SAFETY: вход не ноль (проверено выше), значит bsr определён; читает один
    // регистр в другой, память и стек не трогает.
    unsafe {
        core::arch::asm!(
            "bsr {idx}, {x}",
            idx = out(reg) idx,
            x = in(reg) x,
            options(pure, nomem, nostack),
        );
    }
    63 - idx as u32
}

/// Выбор без ветвления: `cond ? a : b` через `test` + `cmov`. `cmovnz`
/// читает флаг ZF, который только что выставил `test`, — поэтому у блока НЕТ
/// `preserves_flags`: он их портит, и компилятор должен об этом знать.
pub fn select(cond: bool, a: u64, b: u64) -> u64 {
    let out;
    // SAFETY: только регистры и флаги, без памяти и стека. preserves_flags
    // НЕ указан намеренно — test/cmov меняют флаги.
    unsafe {
        core::arch::asm!(
            "test {c}, {c}",
            "cmovnz {b}, {a}",
            c = in(reg) cond as u64,
            a = in(reg) a,
            b = inout(reg) b => out,
            options(pure, nomem, nostack),
        );
    }
    out
}
// ANCHOR_END: solution
