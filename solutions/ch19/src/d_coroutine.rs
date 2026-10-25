//! Задача D: кооперативная сопрограмма на naked-функции переключения
//! контекста. `switch` — тот же приём, что в разделе 19.2, но здесь он
//! применяется в цикле: `main` и сопрограмма по очереди дописывают свой
//! номер в общий журнал, пока не сделают `rounds` обменов.
//!
//! Состояние — модульные `static mut`: сопрограмма входит по C-ABI без
//! аргументов (naked-переключение не переносит замыкания), поэтому контексты,
//! стек и журнал общие. Доступ строго кооперативный и однопоточный — активна
//! всегда ровно одна сторона, гонок нет.

use core::arch::naked_asm;

// ANCHOR: switch
#[repr(C)]
struct Context {
    rsp: u64,
}

/// Сохранить callee-saved регистры текущей стороны на её стек, переключить
/// `rsp` на стек другой и восстановить её регистры. После `ret` исполнение
/// идёт оттуда, где другая сторона ушла в `switch`.
///
/// # Safety
///
/// `from` и `to` указывают на валидные `Context`. `to.rsp` сохранён прошлым
/// `switch` либо указывает на корректно собранный стартовый кадр, кратный
/// 16: после `ret` в точку входа `rsp` ≡ 8 (mod 16), как после `call`. ABI —
/// `extern "C"`: `from` в rdi, `to` в rsi.
#[unsafe(naked)]
unsafe extern "C" fn switch(from: *mut Context, to: *const Context) {
    naked_asm!(
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov [rdi], rsp",
        "mov rsp, [rsi]",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret",
    );
}
// ANCHOR_END: switch

// ANCHOR: state
static mut MAIN: Context = Context { rsp: 0 };
static mut CO: Context = Context { rsp: 0 };
static mut STACK: [u64; 1024] = [0; 1024];
static mut LOG: Vec<u32> = Vec::new();
static mut ROUNDS: u32 = 0;

/// Тело сопрограммы: `rounds` раз дописать 2 и вернуть управление `main`,
/// затем окончательно уйти обратно. Входится по C-ABI без аргументов.
extern "C" fn coroutine() -> ! {
    // SAFETY: активна ровно одна сторона (кооперативно), поэтому единоличный
    // доступ к LOG/ROUNDS/контекстам здесь безопасен.
    unsafe {
        // Через сырой указатель (а не ссылку на static mut): запись в журнал.
        let log = &raw mut LOG;
        for _ in 0..(&raw const ROUNDS).read() {
            (*log).push(2);
            switch(&raw mut CO, &raw const MAIN);
        }
        // Журнал дописан — отдать управление main навсегда.
        switch(&raw mut CO, &raw const MAIN);
    }
    unreachable!("после последнего switch сюда не возвращаются")
}
// ANCHOR_END: state

// ANCHOR: run
/// Прогнать `rounds` обменов: `main` пишет 1, сопрограмма — 2, по очереди.
/// Возвращает журнал порядка исполнения, например для `rounds == 3` это
/// `[1, 2, 1, 2, 1, 2]`.
pub fn pingpong(rounds: u32) -> Vec<u32> {
    // SAFETY: функция кооперативная и однопоточная; переключения идут строго
    // по очереди, поэтому единоличный доступ к общему состоянию соблюдён.
    // Собираем стартовый кадр сопрограммы: шесть callee-saved значений (любых)
    // и адрес возврата = coroutine сверху; первый switch «восстановит» его и
    // ret-нется в coroutine. Верх стека выровнен вниз по 16, над адресом
    // возврата — пустая ячейка: после ret rsp ≡ 8 (mod 16), как ждёт System V
    // ABI, иначе SSE-код в теле сопрограммы (`movaps` по стеку) может упасть.
    unsafe {
        let log = &raw mut LOG;
        (*log).clear();
        (&raw mut ROUNDS).write(rounds);

        let base = (&raw mut STACK).cast::<u64>();
        let sp = base.add(1024).map_addr(|a| a & !15).sub(8);
        *sp.add(6) = coroutine as *const () as u64;
        (&raw mut CO).write(Context { rsp: sp as u64 });

        for _ in 0..rounds {
            (*log).push(1);
            switch(&raw mut MAIN, &raw const CO);
        }
        (*log).clone()
    }
}
// ANCHOR_END: run
