// Общие тесты задач главы 31.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch31/tests/ch31.rs` и `solutions/ch31/tests/ch31.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A проверяется мутантами: каждая пара «стратегия + свойство»
// должна пропустить верную функцию и поймать все её мутанты при одном и
// том же зерне. Задача B — входами, на которых падали внедрённые ошибки,
// и тем, что оракулы молчат на верных входах. Задача C — очередью «после
// почти 2⁶⁴ операций» (`new_at`) против модели. Задача D — вектором из
// описания протокола и прогоном cargo mutants по хранилищу проекта: без
// cargo-mutants этот прогон пропускается, `CH31_REQUIRE_MUTANTS` делает
// пропуск ошибкой.

mod task_a {
    use super::sut::a_props::{self as a, BitOp, Bits, BitSet};
    use proptest::strategy::BoxedStrategy;
    use proptest::test_runner::{Config, RngAlgorithm, TestCaseResult, TestRng, TestRunner};
    use std::fmt::Debug;

    /// Прогон свойства: 256 случаев, зерно одно на все прогоны, без файлов
    /// регрессий. `Err` — текст первой ошибки после уменьшения.
    fn verdict<T: Clone + Debug>(
        s: BoxedStrategy<T>,
        prop: impl Fn(T) -> TestCaseResult,
    ) -> Result<(), String> {
        let config = Config { cases: 256, failure_persistence: None, ..Config::default() };
        let rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
        TestRunner::new_with_rng(config, rng).run(&s, prop).map_err(|e| e.to_string())
    }

    /// Верная функция проходит, каждый мутант падает.
    fn judge<F: Copy>(good: F, mutants: &[(&str, F)], run: impl Fn(F) -> Result<(), String>) {
        if let Err(e) = run(good) {
            panic!("верная функция не прошла свойство: {e}");
        }
        let alive: Vec<&str> =
            mutants.iter().filter(|(_, m)| run(*m).is_ok()).map(|(name, _)| *name).collect();
        assert!(alive.is_empty(), "выжили мутанты: {alive:?}");
    }

    // ------------------------------------------------------ A1 checked_sum
    type Sum = fn(&[u32]) -> Option<u32>;

    fn sum_wrapping(xs: &[u32]) -> Option<u32> {
        Some(xs.iter().fold(0u32, |acc, &x| acc.wrapping_add(x)))
    }

    /// Сумма ровно `u32::MAX` считается переполнением.
    fn sum_boundary(xs: &[u32]) -> Option<u32> {
        let s: u64 = xs.iter().map(|&x| u64::from(x)).sum();
        (s < u64::from(u32::MAX)).then_some(s as u32)
    }

    #[test]
    #[cfg_attr(miri, ignore = "сотни случаев proptest на функцию и её мутантов под Miri — минуты")]
    fn a1_checked_sum() {
        let mutants: [(&str, Sum); 2] = [("wrapping", sum_wrapping), ("boundary", sum_boundary)];
        judge(a::checked_sum as Sum, &mutants, |f| {
            verdict(a::sum_inputs(), move |xs: Vec<u32>| a::sum_prop(f, &xs))
        });
    }

    // ----------------------------------------------------- A2 dedup_sorted
    type Dedup = fn(&[i32]) -> Vec<i32>;

    /// Повтор пропускается через один: из трёх равных остаются два.
    fn dedup_pairs(xs: &[i32]) -> Vec<i32> {
        let (mut out, mut i) = (Vec::new(), 0);
        while i < xs.len() {
            out.push(xs[i]);
            i += if xs.get(i + 1) == Some(&xs[i]) { 2 } else { 1 };
        }
        out
    }

    /// «Страж» вместо `Option`: теряет `i32::MIN` в начале.
    fn dedup_sentinel(xs: &[i32]) -> Vec<i32> {
        let (mut out, mut last) = (Vec::new(), i32::MIN);
        for &x in xs {
            if x != last {
                out.push(x);
                last = x;
            }
        }
        out
    }

    #[test]
    #[cfg_attr(miri, ignore = "сотни случаев proptest на функцию и её мутантов под Miri — минуты")]
    fn a2_dedup_sorted() {
        let mutants: [(&str, Dedup); 2] = [("pairs", dedup_pairs), ("sentinel", dedup_sentinel)];
        judge(a::dedup_sorted as Dedup, &mutants, |f| {
            verdict(a::dedup_inputs(), move |xs: Vec<i32>| a::dedup_prop(f, &xs))
        });
    }

    // ------------------------------------------------------------ A3 varint
    type Enc = fn(u32, &mut [u8; 5]) -> usize;
    type Dec = fn(&[u8]) -> Option<(u32, usize)>;

    /// Цикл «пока не ноль» вместо «хотя бы один байт»: у нуля пустая запись.
    fn enc_zero_empty(mut n: u32, out: &mut [u8; 5]) -> usize {
        let mut i = 0;
        while n != 0 {
            let low = (n & 0x7F) as u8;
            n >>= 7;
            out[i] = if n == 0 { low } else { low | 0x80 };
            i += 1;
        }
        i
    }

    /// Принимает неканоническую запись: `80 00` — тоже ноль.
    fn dec_lenient(bytes: &[u8]) -> Option<(u32, usize)> {
        let mut n = 0u32;
        for (i, &b) in bytes.iter().enumerate().take(5) {
            let low = u32::from(b & 0x7F);
            if i == 4 && low > 0x0F {
                return None;
            }
            n |= low << (7 * i);
            if b & 0x80 == 0 {
                return Some((n, i + 1));
            }
        }
        None
    }

    /// Пятый байт не проверяется: биты сверх 32 молча теряются.
    fn dec_overflow(bytes: &[u8]) -> Option<(u32, usize)> {
        let mut n = 0u32;
        for (i, &b) in bytes.iter().enumerate().take(5) {
            let low = u32::from(b & 0x7F);
            n |= low.wrapping_shl(7 * i as u32);
            if b & 0x80 == 0 {
                return (i == 0 || low != 0).then_some((n, i + 1));
            }
        }
        None
    }

    #[test]
    #[cfg_attr(miri, ignore = "сотни случаев proptest на функцию и её мутантов под Miri — минуты")]
    fn a3_varint() {
        let good: (Enc, Dec) = (a::varint_encode, a::varint_decode);
        let mutants: [(&str, (Enc, Dec)); 3] = [
            ("zero_empty", (enc_zero_empty, a::varint_decode)),
            ("lenient", (a::varint_encode, dec_lenient)),
            ("overflow", (a::varint_encode, dec_overflow)),
        ];
        judge(good, &mutants, |(enc, dec)| {
            verdict(a::varint_inputs(), move |(n, bytes): (u32, Vec<u8>)| {
                a::varint_prop(enc, dec, n, &bytes)
            })
        });
    }

    // --------------------------------------------------------- A4 parse_u16
    type Parse = fn(&str) -> Option<u16>;

    fn digits(s: &str) -> bool {
        s.bytes().all(|b| b.is_ascii_digit())
    }

    fn parse_wrapping(s: &str) -> Option<u16> {
        if s.is_empty() || !digits(s) {
            return None;
        }
        Some(s.bytes().fold(0u16, |n, b| n.wrapping_mul(10).wrapping_add(u16::from(b - b'0'))))
    }

    /// Пустая строка — ноль.
    fn parse_empty(s: &str) -> Option<u16> {
        if !digits(s) {
            return None;
        }
        s.bytes().try_fold(0u16, |n, b| n.checked_mul(10)?.checked_add(u16::from(b - b'0')))
    }

    /// Знак `+` пропускается, как у `str::parse`.
    fn parse_plus(s: &str) -> Option<u16> {
        a::parse_u16(s.strip_prefix('+').unwrap_or(s))
    }

    #[test]
    #[cfg_attr(miri, ignore = "сотни случаев proptest на функцию и её мутантов под Miri — минуты")]
    fn a4_parse_u16() {
        let mutants: [(&str, Parse); 3] =
            [("wrapping", parse_wrapping), ("empty", parse_empty), ("plus", parse_plus)];
        judge(a::parse_u16 as Parse, &mutants, |f| {
            verdict(a::u16_inputs(), move |s: String| a::u16_prop(f, &s))
        });
    }

    // ---------------------------------------------------------- A5 align_up
    type Align = fn(usize, usize) -> Option<usize>;

    /// `(addr | mask) + 1`: выровненный адрес уезжает на целый шаг.
    fn align_or_plus_one(addr: usize, align: usize) -> Option<usize> {
        if !align.is_power_of_two() {
            return None;
        }
        (addr | (align - 1)).checked_add(1)
    }

    /// Без проверки переполнения: у верхнего края результат заворачивается.
    fn align_wrapping(addr: usize, align: usize) -> Option<usize> {
        if !align.is_power_of_two() {
            return None;
        }
        let mask = align - 1;
        Some(addr.wrapping_add(mask) & !mask)
    }

    /// Выравнивание не проверяется на степень двойки.
    fn align_any(addr: usize, align: usize) -> Option<usize> {
        let mask = align.wrapping_sub(1);
        Some(addr.checked_add(mask)? & !mask)
    }

    #[test]
    #[cfg_attr(miri, ignore = "сотни случаев proptest на функцию и её мутантов под Miri — минуты")]
    fn a5_align_up() {
        let mutants: [(&str, Align); 3] = [
            ("or_plus_one", align_or_plus_one),
            ("wrapping", align_wrapping),
            ("any_align", align_any),
        ];
        judge(a::align_up as Align, &mutants, |f| {
            verdict(a::align_inputs(), move |(addr, align): (usize, usize)| {
                a::align_prop(f, addr, align)
            })
        });
    }

    // ------------------------------------------------------------- A6 Bits
    /// Обёртка над верным множеством, где одну операцию подменяет мутант.
    #[derive(Default)]
    struct Mutant<const M: u8>(BitSet, [u64; 4]);

    impl<const M: u8> Bits for Mutant<M> {
        fn insert(&mut self, i: u8) -> bool {
            self.1[usize::from(i / 64)] |= 1 << (i % 64);
            let new = self.0.insert(i);
            // insert_always_new: ответ «не было» всегда.
            if M == 3 { true } else { new }
        }

        fn remove(&mut self, i: u8) -> bool {
            let had = self.0.contains(i);
            if M == 1 {
                // toggle_remove: `^=` вместо `&= !` — удаление отсутствующего
                // числа его добавляет.
                if had { self.0.remove(i) } else { !self.0.insert(i) }
            } else {
                self.0.remove(i)
            }
        }

        fn contains(&self, i: u8) -> bool {
            self.0.contains(i)
        }

        fn len(&self) -> usize {
            if M == 2 {
                // short_len: последнее слово (числа 192–255) не считается.
                (0..192).filter(|&i| self.0.contains(i as u8)).count()
            } else {
                self.0.len()
            }
        }
    }

    /// Как построить проверяемое множество: верное или мутанта.
    type MakeBits = fn() -> Box<dyn Bits>;

    fn bits_verdict(make: MakeBits) -> Result<(), String> {
        verdict(a::bits_inputs(), move |ops: Vec<BitOp>| a::bits_prop(&mut *make(), &ops))
    }

    #[test]
    #[cfg_attr(miri, ignore = "сотни случаев proptest на функцию и её мутантов под Miri — минуты")]
    fn a6_bitset() {
        let mutants: [(&str, MakeBits); 3] = [
            ("toggle_remove", || Box::new(Mutant::<1>::default())),
            ("short_len", || Box::new(Mutant::<2>::default())),
            ("insert_always_new", || Box::new(Mutant::<3>::default())),
        ];
        judge(|| Box::new(BitSet::default()) as Box<dyn Bits>, &mutants, bits_verdict);
    }
}

mod task_b {
    use super::sut::b_fuzz::{check_config, check_packet, ip, lexer};

    /// Пример заголовка из статьи Wikipedia «IPv4 header checksum».
    const WIKI_HEADER: [u8; 20] = [
        0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0xb8, 0x61, 0xc0, 0xa8, 0x00,
        0x01, 0xc0, 0xa8, 0x00, 0xc7,
    ];

    /// SYN 192.168.0.1:12345 → 192.168.0.2:80 с опцией MSS 1460, собран scapy.
    const SYN: [u8; 44] = [
        0x45, 0x00, 0x00, 0x2c, 0x1c, 0x46, 0x40, 0x00, 0x40, 0x06, 0x9d, 0x32, 0xc0, 0xa8, 0x00,
        0x01, 0xc0, 0xa8, 0x00, 0x02, 0x30, 0x39, 0x00, 0x50, 0x01, 0x02, 0x03, 0x04, 0x00, 0x00,
        0x00, 0x00, 0x60, 0x02, 0xfa, 0xf0, 0xe7, 0x52, 0x00, 0x00, 0x02, 0x04, 0x05, 0xb4,
    ];

    /// Пакет из примера Wikipedia: заголовок и нули до Total Length = 0x73.
    fn wiki_packet() -> Vec<u8> {
        let mut p = WIKI_HEADER.to_vec();
        p.resize(0x73, 0);
        p
    }

    /// Ошибка заготовки в ip: дополнение кадра Ethernet за Total Length
    /// попадало в нагрузку.
    #[test]
    fn b_ipv4_padding_is_not_payload() {
        let mut p = wiki_packet();
        p.extend_from_slice(&[0xAA; 6]);
        let h = ip::parse_ipv4(&p).unwrap();
        assert_eq!(h.payload.len(), 0x73 - 20);
        let mut syn = SYN.to_vec();
        syn.extend_from_slice(&[0; 16]);
        let h = ip::parse_ipv4(&syn).unwrap();
        let t = ip::parse_tcp(h.payload).unwrap();
        assert!(t.payload.is_empty(), "нагрузка SYN: {:?}", t.payload);
    }

    /// Ошибка заготовки в lexer: позиция комментария в символах резала
    /// строку по байтам — паника на границе символа или обрезанное
    /// значение.
    #[test]
    fn b_lexer_multibyte_before_comment() {
        use lexer::Token::{Key, Value};
        let tokens = |s| lexer::Lexer::new(s).collect::<Vec<_>>();
        assert_eq!(tokens("name = ёж ёж # комментарий"), [Ok(Key("name")), Ok(Value("ёж ёж"))]);
        assert_eq!(tokens("t = 25 °C ; c"), [Ok(Key("t")), Ok(Value("25 °C"))]);
    }

    /// Псевдослучайные байты для оракулов: xorshift с фиксированным зерном.
    fn noise(seed: u64, n: usize) -> Vec<u8> {
        let mut x = seed | 1;
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect()
    }

    /// На верных пакетах, их порче и шуме оракул молчит: ложных тревог нет.
    #[test]
    fn b_check_packet_accepts_valid() {
        let mut inputs = vec![wiki_packet(), SYN.to_vec(), SYN[20..].to_vec(), Vec::new()];
        let mut padded = wiki_packet();
        padded.extend_from_slice(&[0xAA; 6]);
        inputs.push(padded);
        for i in 0..SYN.len() {
            inputs.push(SYN[..i].to_vec());
            for bit in 0..8 {
                let mut p = SYN.to_vec();
                p[i] ^= 1 << bit;
                inputs.push(p);
            }
        }
        for seed in 1..200 {
            inputs.push(noise(seed, (seed as usize * 7) % 90));
        }
        for p in &inputs {
            check_packet(p);
        }
    }

    const CONFIG: &str = "\
# станция телеметрии
[uart]
baud = 9600   # скорость
mode=8N1
name = \"метео; 3\"
motd = \"  в кавычках пробелы свои  \"
place = Обнинск, крыша ; координаты потом
\t[spi]
clock = 1000000
empty =
[bad
=1
x = \"не закрыта
";

    /// На верном тексте, его обрезках и шуме оракул молчит.
    #[test]
    fn b_check_config_accepts_valid() {
        check_config(CONFIG);
        check_config("");
        for (i, _) in CONFIG.char_indices() {
            check_config(&CONFIG[..i]);
            check_config(&CONFIG[i..]);
        }
        for seed in 1..200 {
            check_config(&String::from_utf8_lossy(&noise(seed, 60)));
        }
    }
}

mod task_c {
    use super::sut::c_ring::Ring;
    use std::collections::VecDeque;

    /// Случайные push и pop против `VecDeque` с ёмкостью `N`, начиная с
    /// состояния после `start` операций, потом всё забрать.
    fn against_model<const N: usize>(start: usize, seed: u64) {
        let q = Ring::<u64, N>::new_at(start);
        let mut model = VecDeque::new();
        let (mut x, mut next) = (seed | 1, 0u64);
        for _ in 0..8 * N + 8 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            if x % 3 != 0 {
                let got = q.push(next);
                if model.len() < N {
                    assert_eq!(got, Ok(()), "N = {N}, start = {start:#x}");
                    model.push_back(next);
                } else {
                    assert_eq!(got, Err(next), "N = {N}: push в полную очередь");
                }
                next += 1;
            } else {
                assert_eq!(q.pop(), model.pop_front(), "N = {N}, start = {start:#x}");
            }
            assert_eq!(q.len(), model.len());
        }
        while let Some(v) = model.pop_front() {
            assert_eq!(q.pop(), Some(v), "N = {N}, start = {start:#x}");
        }
        assert_eq!(q.pop(), None);
        assert!(q.is_empty());
    }

    fn all_starts<const N: usize>() {
        let starts = [0, 1, 1000, usize::MAX / 2, usize::MAX - 3 * N, usize::MAX - 1, usize::MAX];
        for (i, &start) in starts.iter().enumerate() {
            for seed in 1..20 {
                against_model::<N>(start, seed * 1000 + i as u64);
            }
        }
    }

    #[test]
    fn c_fifo_for_every_size_up_to_8() {
        all_starts::<1>();
        all_starts::<2>();
        all_starts::<3>();
        all_starts::<4>();
        all_starts::<5>();
        all_starts::<6>();
        all_starts::<7>();
        all_starts::<8>();
    }

    /// Все `N` слотов заняты: очередь на 3 держит 3 элемента.
    #[test]
    fn c_uses_all_slots() {
        let q = Ring::<u8, 3>::new();
        assert_eq!((q.push(1), q.push(2), q.push(3), q.push(4)), (Ok(()), Ok(()), Ok(()), Err(4)));
        assert_eq!(q.len(), 3);
    }

    /// Писатель и читатель в разных потоках проходят через заворот. Если
    /// писатель упал, читатель не ждёт вечно: он дочитывает очередь после
    /// `join` и передаёт панику дальше.
    #[test]
    #[cfg_attr(miri, ignore = "сто тысяч элементов под Miri слишком долго")]
    fn c_threads_across_wrap() {
        let q = Ring::<u32, 3>::new_at(usize::MAX - 50_000);
        std::thread::scope(|s| {
            let producer = s.spawn(|| {
                for i in 0..100_000 {
                    while q.push(i).is_err() {
                        std::hint::spin_loop();
                    }
                }
            });
            let mut want = 0;
            while want < 100_000 && !producer.is_finished() {
                if let Some(v) = q.pop() {
                    assert_eq!(v, want);
                    want += 1;
                }
            }
            // После join всё записанное писателем видно читателю.
            let result = producer.join();
            while let Some(v) = q.pop() {
                assert_eq!(v, want);
                want += 1;
            }
            if let Err(panic) = result {
                std::panic::resume_unwind(panic);
            }
            assert_eq!(want, 100_000);
        });
    }

    #[test]
    fn c_drop_releases_items() {
        let item = std::rc::Rc::new(());
        let q = Ring::<_, 4>::new_at(usize::MAX - 1);
        for _ in 0..3 {
            q.push(item.clone()).unwrap();
        }
        assert_eq!(std::rc::Rc::strong_count(&item), 4);
        drop(q);
        assert_eq!(std::rc::Rc::strong_count(&item), 1);
    }
}

mod task_d {
    use super::sut::d_spec::{TimeSync, decode, encode};
    use std::path::Path;
    use std::process::Command;

    /// Пример из описания протокола: полночь 1 января 2026 года UTC с
    /// поправкой −250 мс.
    #[test]
    fn d_spec_example() {
        let s = TimeSync { epoch_s: 1_767_225_600, offset_ms: -250 };
        assert_eq!(encode(s), [0x69, 0x55, 0xB9, 0x00, 0xFF, 0x06]);
        assert_eq!(decode(&[0x69, 0x55, 0xB9, 0x00, 0xFF, 0x06]), Some(s));
    }

    /// cargo mutants по хранилищу проекта; тесты — его собственные
    /// (`tests/store.rs`) и `tests/d_store.rs` этого крейта: выживших быть не
    /// должно. Остальные тесты крейта не запускаются — среди них этот же.
    /// Свой прогон без мутаций cargo-mutants 27 делает только с пакетом
    /// хранилища, а не с `--test-package`, и на фильтре `--test d_store` он
    /// падает. Поэтому он выключен (`--baseline skip`), а чистое дерево тест
    /// проверяет сам, в отдельном каталоге сборки: иначе тест, падающий
    /// всегда, «убил» бы всех мутантов.
    #[test]
    #[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
    fn d_store_mutants_killed() {
        let require = std::env::var_os("CH31_REQUIRE_MUTANTS").is_some();
        let have = Command::new("cargo").args(["mutants", "--version"]).output();
        if !have.is_ok_and(|o| o.status.success()) {
            assert!(!require, "нет cargo-mutants");
            eprintln!("пропуск: нет cargo-mutants (cargo install cargo-mutants)");
            return;
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{}-mutants", env!("CARGO_PKG_NAME")));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        let cargo = || {
            let mut cmd = Command::new("cargo");
            cmd.current_dir(&root).env("TMPDIR", &out);
            for v in ["CARGO_ENCODED_RUSTFLAGS", "RUSTFLAGS", "CARGO_TARGET_DIR"] {
                cmd.env_remove(v);
            }
            cmd
        };
        let clean = cargo()
            .args(["test", "-q", "-p", "telemetry-store", "-p", env!("CARGO_PKG_NAME")])
            .args(["--test", "store", "--test", "d_store", "--target-dir"])
            .arg(out.join("clean"))
            .output()
            .unwrap();
        assert!(
            clean.status.success(),
            "тесты хранилища падают и без мутаций:\n{}{}",
            String::from_utf8_lossy(&clean.stdout),
            String::from_utf8_lossy(&clean.stderr)
        );
        let _ = std::fs::remove_dir_all(out.join("clean"));
        let run = cargo()
            .args(["mutants", "-p", "telemetry-store", "--test-package"])
            .arg(concat!("telemetry-store,", env!("CARGO_PKG_NAME")))
            .args(["--baseline", "skip", "--timeout", "60", "--build-timeout", "600"])
            .args(["--gitignore", "true", "--no-shuffle", "-j", "4", "-o"])
            .arg(&out)
            .args(["--", "--test", "store", "--test", "d_store"])
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&run.stdout).into_owned()
            + &String::from_utf8_lossy(&run.stderr);
        println!("{log}");
        let missed = std::fs::read_to_string(out.join("mutants.out/missed.txt")).unwrap_or_default();
        let caught = std::fs::read_to_string(out.join("mutants.out/caught.txt")).unwrap_or_default();
        assert!(caught.lines().count() > 0, "cargo mutants ничего не поймал:\n{log}");
        assert!(missed.is_empty(), "выжили мутанты хранилища:\n{missed}");
    }
}
