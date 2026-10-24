//! Раздел 26.2: процесс Linux без libc. Первая инструкция процесса —
//! наш `_start`; он передаёт в Rust указатель на то, что ядро положило
//! на стек: argc, argv, envp и вспомогательный вектор auxv.
#![no_std]
#![no_main]

use core::arch::naked_asm;

// Тот же модуль, что в библиотеке ch26; программе нужна не вся его часть.
#[allow(dead_code)]
#[path = "../../src/sys.rs"]
mod sys;

// Опыт с выравниванием: с возможностью misalign `_start` сдвигает стек
// на 8 байт, и Rust-код получает rsp, сдвинутый от того, что обещает ABI.
#[cfg(not(feature = "misalign"))]
macro_rules! misalign {
    () => {
        ""
    };
}
#[cfg(feature = "misalign")]
macro_rules! misalign {
    () => {
        "sub rsp, 8"
    };
}

// ANCHOR: start
/// Точка входа. Ядро передаёт управление сюда со стеком, выровненным на
/// 16 байт, и `rsp` указывает на argc. Функция naked: пролога нет, и
/// стек остаётся таким, каким его оставило ядро.
///
/// # Safety
///
/// Вызывает только ядро, один раз, при запуске процесса.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        // Конец цепочки кадров для отладчика: rbp = 0.
        "xor ebp, ebp",
        // Первый аргумент main_from_start — указатель на argc.
        "mov rdi, rsp",
        // Пусто; с возможностью misalign — "sub rsp, 8".
        misalign!(),
        // call кладёт 8 байт адреса возврата: в вызванной функции
        // rsp + 8 кратно 16, как требует System V ABI.
        "call {main}",
        "ud2",
        main = sym main_from_start,
    )
}
// ANCHOR_END: start

// ANCHOR: main
/// Номера записей auxv, которые печатает программа (`<elf.h>`).
const AT_PAGESZ: usize = 6;
const AT_ENTRY: usize = 9;
const AT_RANDOM: usize = 25;

/// Стек при входе в процесс, снизу вверх: argc, argv[0..argc], NULL,
/// envp[..], NULL, пары (тип, значение) auxv до AT_NULL = 0.
unsafe extern "C" fn main_from_start(sp: *const usize) -> ! {
    // SAFETY: раскладку стека задаёт ядро (System V ABI, раздел 3.4.1);
    // все указатели до AT_NULL — на отображённую память процесса.
    unsafe {
        let argc = *sp;
        let argv = sp.add(1) as *const *const u8;
        let envp = argv.add(argc + 1);
        let mut envc = 0;
        while !(*envp.add(envc)).is_null() {
            envc += 1;
        }
        let mut aux = envp.add(envc + 1) as *const [usize; 2];

        out("argc = ");
        num(argc);
        out("\n");
        for i in 0..argc {
            let arg = *argv.add(i);
            out("argv[");
            num(i);
            out("] = ");
            out(core::slice::from_raw_parts(arg, strlen(arg)));
            out("\n");
        }
        out("переменных окружения: ");
        num(envc);
        out("\n");

        while (*aux)[0] != 0 {
            let [kind, value] = *aux;
            match kind {
                AT_PAGESZ => {
                    out("AT_PAGESZ = ");
                    num(value);
                    out("\n");
                }
                AT_ENTRY => {
                    out("AT_ENTRY = ");
                    hex(value);
                    out(", _start = ");
                    hex(_start as *const () as usize);
                    out("\n");
                }
                AT_RANDOM => {
                    out("AT_RANDOM: 16 случайных байт по адресу ");
                    hex(value);
                    out("\n");
                }
                _ => {}
            }
            aux = aux.add(1);
        }
        out("rsp при входе в _start: ");
        hex(sp as usize);
        let aligned = (sp as usize).is_multiple_of(16);
        out(if aligned { ", кратно 16\n" } else { ", НЕ кратно 16\n" });
    }
    sys::exit(0)
}
// ANCHOR_END: main

fn out(b: impl AsRef<[u8]>) {
    if sys::write_all(1, b.as_ref()).is_err() {
        sys::exit(2);
    }
}

fn num(mut x: usize) {
    let mut buf = [0u8; 20];
    let mut i = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (x % 10) as u8;
        x /= 10;
        if x == 0 {
            break;
        }
    }
    out(&buf[i..]);
}

fn hex(x: usize) {
    let mut buf = *b"0x0000000000000000";
    for i in 0..16 {
        let d = (x >> (60 - 4 * i)) & 0xF;
        buf[2 + i] = b"0123456789abcdef"[d];
    }
    out(buf);
}

// ANCHOR: strlen
/// Компилятор сам вставляет вызовы `strlen`, `memcpy` и `memset`
/// (раздел 26.5), а libc, которая их даёт, мы не подключаем. Цикл в
/// функции с именем `strlen` LLVM вызовом самой себя не заменяет.
///
/// # Safety
///
/// `p` указывает на строку C, оканчивающуюся нулём.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(p: *const u8) -> usize {
    let mut n = 0;
    // SAFETY: до нулевого байта память читаема — договор вызывающего.
    while unsafe { *p.add(n) } != 0 {
        n += 1;
    }
    n
}
// ANCHOR_END: strlen

// ANCHOR: panic
/// Без std паника не раскручивает стек и ничего не печатает: этот
/// обработчик завершает процесс с кодом 101, как необработанная паника
/// в программе со std.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    sys::exit(101)
}
// ANCHOR_END: panic
