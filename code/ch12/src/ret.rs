//! Раздел 12.4: замыкание как результат функции и как поле структуры.

// ANCHOR: counter
/// Счётчик, который помнит состояние между вызовами. Тип результата назван
/// не по имени, а по свойству: «что-то, что реализует `FnMut() -> u32`».
pub fn make_counter(start: u32) -> impl FnMut() -> u32 {
    let mut n = start;
    move || {
        n += 1;
        n
    }
}
// ANCHOR_END: counter

/// Без `move` замыкание держало бы ссылку на `n`, а `n` умирает вместе с
/// кадром `make_counter`. Stable 1.97 сообщает об этом как E0373, nightly
/// 1.101 — как E0597, поэтому код ошибки здесь не проверяется.
///
/// ```compile_fail
/// fn make_counter(start: u32) -> impl FnMut() -> u32 {
///     let mut n = start;
///     || {
///         n += 1;
///         n
///     }
/// }
/// ```
pub const fn dangling() {}

// ANCHOR: either
/// Два замыкания без захвата — два разных типа, но общий тип у них есть:
/// указатель на функцию.
pub fn pick_fn(invert: bool) -> fn(u8) -> u8 {
    if invert { |x| !x } else { |x| x }
}

/// С захватом общего типа нет. Одно замыкание с ветвлением внутри
/// обходится без кучи...
pub fn pick_branch(invert: bool, k: u8) -> impl Fn(u8) -> u8 {
    move |x| if invert { !x ^ k } else { x ^ k }
}

/// ...а `Box<dyn Fn>` стирает тип ценой выделения памяти и косвенного
/// вызова.
#[cfg(feature = "alloc")]
pub fn pick_boxed(invert: bool, k: u8) -> alloc::boxed::Box<dyn Fn(u8) -> u8> {
    use alloc::boxed::Box;
    if invert { Box::new(move |x| !x ^ k) } else { Box::new(move |x| x ^ k) }
}
// ANCHOR_END: either

/// `impl Fn` — один скрытый тип, а у двух замыканий типы разные, даже
/// если тела одинаковы.
///
/// ```compile_fail,E0308
/// fn pick(invert: bool, k: u8) -> impl Fn(u8) -> u8 {
///     if invert { move |x: u8| !x ^ k } else { move |x: u8| x ^ k }
/// }
/// ```
pub const fn two_types() {}

// ANCHOR: compose
/// Композиция: результат захватывает оба аргумента по значению.
pub fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

/// Калибровка из трёх шагов. Её тип — вложенные замыкания, размер — сумма
/// захваченного: `offset` и `gain`, восемь байт.
pub fn pipeline(offset: i32, gain: i32) -> impl Fn(i32) -> i32 {
    let shift = move |x| x - offset;
    let scale = move |x| x * gain;
    compose(compose(shift, scale), |x: i32| x.clamp(-1000, 1000))
}
// ANCHOR_END: compose

// ANCHOR: use-bound
/// В редакции 2024 `impl Trait` в результате захватывает все параметры
/// времени жизни функции, даже если скрытый тип ими не пользуется.
/// `use<>` перечисляет захваченное явно, здесь — ничего: замыкание держит
/// копию `k`, а не ссылку.
pub fn scaler(k: &i32) -> impl Fn(i32) -> i32 + use<> {
    let k = *k;
    move |x| x * k
}
// ANCHOR_END: use-bound

/// Без `use<>` результат считается заёмом `k`, и менять `k`, пока он жив,
/// нельзя, хотя замыкание ссылки не держит.
///
/// ```compile_fail,E0506
/// fn scaler(k: &i32) -> impl Fn(i32) -> i32 {
///     let k = *k;
///     move |x| x * k
/// }
/// let mut k = 3;
/// let f = scaler(&k);
/// k += 1;
/// assert_eq!(f(2), 6);
/// ```
///
/// А если замыкание ссылку держит, `use<>` не даст об этом умолчать.
///
/// ```compile_fail,E0700
/// fn scaler(k: &i32) -> impl Fn(i32) -> i32 + use<> {
///     move |x| x * *k
/// }
/// ```
pub const fn capture_rules() {}

// ANCHOR: field
/// Замыкание в поле структуры: параметр типа, как в главе 10. Подавитель
/// дребезга кнопки сообщает о новом уровне после трёх одинаковых отсчётов
/// подряд.
pub struct Debounce<F: FnMut(bool)> {
    level: bool,
    same: u8,
    on_change: F,
}

impl<F: FnMut(bool)> Debounce<F> {
    pub fn new(on_change: F) -> Self {
        Debounce { level: false, same: 0, on_change }
    }

    pub fn sample(&mut self, level: bool) {
        if level == self.level {
            self.same = 0;
            return;
        }
        self.same += 1;
        if self.same == 3 {
            self.level = level;
            self.same = 0;
            (self.on_change)(level);
        }
    }
}
// ANCHOR_END: field

/// Поле-замыкание вызывают в скобках: `self.on_change(level)` — это поиск
/// метода.
///
/// ```compile_fail,E0599
/// struct Button<F: FnMut()> { on_click: F }
/// impl<F: FnMut()> Button<F> {
///     fn press(&mut self) { self.on_click() }
/// }
/// ```
pub const fn field_call() {}

/// Граница нужна там, где создают замыкание. Если `new` лежит в `impl`
/// без границы, тип параметра замыкания не из чего вывести: граница на
/// `press` вступает в силу позже, чем компилятор проверяет тело.
///
/// ```compile_fail,E0282
/// struct Button<F> { on_press: F }
/// impl<F> Button<F> {
///     fn new(on_press: F) -> Self { Button { on_press } }
/// }
/// impl<F: FnMut(bool)> Button<F> {
///     fn press(&mut self, long: bool) { (self.on_press)(long) }
/// }
/// let mut longs = 0u32;
/// let mut b = Button::new(|long| longs += long.then_some(1).unwrap_or(0));
/// b.press(true);
/// ```
pub const fn bound_on_new() {}

#[cfg(test)]
mod tests {
    use super::*;

    const W: usize = size_of::<usize>();

    #[test]
    fn returned_closures() {
        let mut c = make_counter(10);
        assert_eq!([c(), c(), c()], [11, 12, 13]);
        assert_eq!(size_of_val(&c), 4);
        assert_eq!([pick_fn(true)(0x0f), pick_fn(false)(0x0f)], [0xf0, 0x0f]);
        assert_eq!([pick_branch(true, 1)(0x0f), pick_branch(false, 1)(0x0f)], [0xf1, 0x0e]);
        #[cfg(feature = "alloc")]
        {
            let b = pick_boxed(true, 1);
            assert_eq!(b(0x0f), 0xf1);
            assert_eq!(size_of_val(&b), 2 * W);
        }
        let p = pipeline(100, 3);
        assert_eq!([p(90), p(200), p(1000)], [-30, 300, 1000]);
        assert_eq!(size_of_val(&p), 8);
        let mut k = 3;
        let f = scaler(&k);
        k += 1;
        assert_eq!((f(2), k), (6, 4));
    }

    #[test]
    fn closure_in_field() {
        let mut changes = 0u32;
        let mut last = false;
        {
            let mut d = Debounce::new(|l| {
                changes += 1;
                last = l;
            });
            for l in [true, false, true, true, true, true, false, false, false] {
                d.sample(l);
            }
        }
        assert_eq!((changes, last), (2, false));
    }

    /// Три слова — раскладка rustc 1.97: `bool` и `u8` лежат рядом, а не
    /// между ссылками. Порядок полей `repr(Rust)` не обещан, и под
    /// -Zrandomize-layout размер бывает больше, поэтому тест наблюдательный.
    #[test]
    fn observed_debounce_size() {
        let (mut a, mut b) = (0u8, 0u8);
        let d = Debounce::new(|l: bool| {
            a += 1;
            b = u8::from(l);
        });
        assert_eq!(size_of_val(&d), 3 * W);
    }
}
