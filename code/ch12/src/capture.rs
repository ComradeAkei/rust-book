//! Раздел 12.1: синтаксис замыканий, три способа захвата, точный захват
//! полей и размер замыкания.

// ANCHOR: syntax
/// Одно и то же сложение тремя записями: типы параметров и результата
/// можно указать, а можно оставить выводу.
pub fn syntax() -> [i32; 3] {
    let add_typed = |x: i32, y: i32| -> i32 { x + y };
    let add = |x, y| x + y;
    let answer = || 42;
    [add_typed(1, 2), add(3, 4), answer()]
}
// ANCHOR_END: syntax

/// Тип параметра замыкания без аннотации выводится из первого вызова и
/// дальше не меняется: замыкание не обобщённая функция.
///
/// ```compile_fail,E0308
/// let id = |x| x;
/// let a = id(1u8);
/// let b = id(1u16);
/// ```
pub const fn inference_is_fixed() {}

// ANCHOR: modes
/// Компилятор выбирает самый слабый способ захвата, которого хватает телу:
/// ссылку, если тело только читает, изменяемую ссылку, если изменяет, и
/// значение, если отдаёт захваченное.
#[cfg(feature = "alloc")]
pub fn modes() -> (usize, u32, alloc::string::String) {
    let name = alloc::string::String::from("термопара");
    let mut count = 0u32;

    let len = || name.len(); //         захват &name
    let mut bump = || count += 1; //    захват &mut count
    bump();
    bump();
    let n = len();
    let take = || name; //               захват name по значению
    (n, count, take())
}
// ANCHOR_END: modes

/// Пока жива изменяемая ссылка внутри `bump`, читать `count` нельзя:
/// замыкание — заём, и у него обычные правила заимствования.
///
/// ```compile_fail,E0503
/// let mut count = 0;
/// let mut bump = || count += 1;
/// let seen = count;
/// bump();
/// ```
///
/// Замыкание, которое отдаёт захваченную строку, забирает её себе, и
/// после его создания строки у функции нет.
///
/// ```compile_fail,E0382
/// let name = String::from("термопара");
/// let take = || name;
/// let len = name.len();
/// take();
/// ```
pub const fn borrow_rules() {}

// ANCHOR: desugar
/// Что компилятор строит из `|| count += 1`: структуру с полем на каждую
/// захваченную переменную и метод, тело которого — тело замыкания. Имени у
/// настоящего типа нет, `Bump` — только модель.
pub struct Bump<'a> {
    count: &'a mut u32,
}

impl Bump<'_> {
    pub fn call_mut(&mut self) {
        *self.count += 1;
    }
}

pub fn desugared() -> u32 {
    let mut count = 0;
    let mut bump = Bump { count: &mut count };
    bump.call_mut();
    bump.call_mut();
    count
}
// ANCHOR_END: desugar

// ANCHOR: sizes
fn size<F>(_: &F) -> usize {
    size_of::<F>()
}

/// Размер замыкания — размер его структуры: сумма захваченного с
/// выравниванием.
pub fn sizes() -> [usize; 4] {
    let x = 5i32;
    let mut y = 7u8;
    let z = 9u64;
    let nothing = || 1;
    let by_ref = || x + 1;
    let by_value = move || x + 1;
    let mut two_refs = || {
        y += 1;
        z
    };
    let sizes = [size(&nothing), size(&by_ref), size(&by_value), size(&two_refs)];
    let _ = (nothing(), by_ref(), by_value(), two_refs());
    sizes
}
// ANCHOR_END: sizes

// ANCHOR: precise
pub struct Pair {
    pub a: u64,
    pub b: u64,
    pub log: [u8; 100],
}

/// С редакции 2021 захватывается не переменная, а путь к тому, чем тело
/// пользуется: поле, поле поля, разыменование ссылки.
pub fn precise_sizes() -> [usize; 3] {
    let p = Pair { a: 1, b: 2, log: [0; 100] };
    let fields = || p.a + p.b; //             &p.a и &p.b
    let whole = || {
        let q = &p; //                         &p целиком
        q.a + q.b
    };
    let copied = move || p.a + p.b; //        копии p.a и p.b
    let sizes = [size(&fields), size(&whole), size(&copied)];
    let _ = (fields(), whole(), copied(), p.log[0]);
    sizes
}
// ANCHOR_END: precise

// ANCHOR: disjoint
pub struct Station {
    pub name: &'static str,
    pub count: u32,
}

/// Изменяемая ссылка на одно поле и замыкание, читающее другое, живут
/// одновременно: замыкание захватило `st.name`, а не `st`.
pub fn disjoint(st: &mut Station) -> usize {
    let r = &mut st.count;
    let len = || st.name.len();
    *r += 1;
    len()
}
// ANCHOR_END: disjoint

/// В редакции 2018 то же самое не компилируется: замыкание захватывает
/// переменную `st` целиком.
///
/// ```compile_fail,edition2018,E0502
/// struct Station { name: &'static str, count: u32 }
/// fn disjoint(st: &mut Station) -> usize {
///     let r = &mut st.count;
///     let len = || st.name.len();
///     *r += 1;
///     len()
/// }
/// ```
pub const fn edition_2018() {}

// ANCHOR: move-copy
/// `move` копирует `Copy`-значение в замыкание. Счётчик растёт в копии, а
/// переменная функции остаётся нулём, и ни rustc, ни clippy об этом не
/// предупреждают.
pub fn move_copy() -> (u32, u32) {
    let mut n = 0u32;
    let mut inc = move || {
        n += 1;
        n
    };
    inc();
    inc();
    let inside = inc();
    (n, inside)
}
// ANCHOR_END: move-copy

// ANCHOR: move-ref
/// Чтобы `move`-замыкание взяло что-то по ссылке, переместите в него саму
/// ссылку: ссылка — `Copy`, и её копия указывает туда же.
pub fn move_ref() -> (usize, usize) {
    let samples = [3u16, 5, 8, 13];
    let limit = 4u16;
    let r = &samples;
    let above = move || r.iter().filter(|&&s| s > limit).count();
    (above(), size(&above))
}
// ANCHOR_END: move-ref

#[cfg(test)]
mod tests {
    use super::*;

    const W: usize = size_of::<usize>();

    #[test]
    fn closures_work() {
        assert_eq!(syntax(), [3, 7, 42]);
        #[cfg(feature = "alloc")]
        assert_eq!(modes(), ("термопара".len(), 2, "термопара".into()));
        assert_eq!(desugared(), 2);
        let mut st = Station { name: "станция", count: 0 };
        assert_eq!(disjoint(&mut st), "станция".len());
        assert_eq!(st.count, 1);
    }

    #[test]
    fn sizes_are_captures() {
        assert_eq!(sizes(), [0, W, 4, 2 * W]);
        assert_eq!(precise_sizes(), [2 * W, W, 16]);
        assert_eq!(move_ref(), (3, 2 * W));
    }

    #[test]
    fn move_copies() {
        assert_eq!(move_copy(), (0, 3));
    }
}
