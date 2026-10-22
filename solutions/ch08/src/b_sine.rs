//! Задача B: синус в фиксированной точке Q15 для Cortex-M0, у которого нет
//! FPU. Таблицу генерирует `build.rs`, здесь только целочисленный код.

// ANCHOR: include
// pub static SINE: [i16; 257] — четверть периода, от 0 до π/2.
include!(concat!(env!("OUT_DIR"), "/sine.rs"));
// ANCHOR_END: include

// ANCHOR: sin
/// Синус угла `angle` в формате Q15 (32767 — это 1.0). Полный оборот —
/// 65536 единиц, поэтому угол переполняется ровно на 360°.
#[inline(never)]
pub fn sin_q15(angle: u16) -> i16 {
    let quadrant = angle >> 14;
    let a = angle & 0x3fff;
    // Во второй и четвёртой четвертях синус идёт по таблице обратно:
    // sin(90° + x) = sin(90° − x). Отсюда a в 1..=0x4000 и 257-я точка.
    let a = if quadrant & 1 == 1 { 0x4000 - a } else { a };
    let i = usize::from(a >> 6); // 256 отрезков по 64 единицы
    let frac = i32::from(a & 0x3f);
    let y0 = i32::from(SINE[i]);
    // На последней точке frac = 0, и соседняя не нужна.
    let y1 = i32::from(SINE[(i + 1).min(256)]);
    // Линейная интерполяция с округлением: синус на четверти растёт,
    // y1 − y0 ≥ 0, сдвиг вправо округляет к ближайшему.
    let y = y0 + (((y1 - y0) * frac + 32) >> 6);
    if quadrant >= 2 { -y as i16 } else { y as i16 }
}
// ANCHOR_END: sin
