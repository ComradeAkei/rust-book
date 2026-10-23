//! Драйвер для теста кодогенерации (`tests/asm.rs`): вызывает примитивы
//! главы, чтобы их инструкции наверняка попали в объектный код. Без вызова
//! rustc хранит маленькую `asm!`-функцию как MIR для межкрейтового инлайна, и
//! в ассемблере самой библиотеки её инструкций нет. `black_box` не даёт
//! свернуть вызовы в константы.

use std::hint::black_box;

fn main() {
    // 19.1: asm! — rdtsc/lfence и imul/add.
    black_box(ch19::asm::rdtsc());
    black_box(ch19::asm::muladd(black_box(3), black_box(4), black_box(5)));

    // 19.3: интринсики — paddd (SSE2) и vpaddd (AVX2).
    let data = black_box([1i32, 2, 3, 4, 5, 6, 7, 8, 9]);
    black_box(ch19::simd::sum_sse2(black_box(&data)));
    // Вызов в коде всегда компилируется (vpaddd появляется в .s), а на
    // процессоре без AVX2 просто не исполняется.
    if is_x86_feature_detected!("avx2") {
        // SAFETY: фича проверена прямо перед вызовом.
        black_box(unsafe { ch19::simd::sum_avx2(black_box(&data)) });
    }
}
