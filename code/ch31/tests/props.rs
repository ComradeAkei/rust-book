//! Раздел 31.3: свойства и стратегии proptest — круговой обход, модель,
//! устойчивость к порче, уменьшение контрпримера, файлы регрессий.

use ch31::basics::Window;
use ch31::slip;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::Index;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, FileFailurePersistence, RngAlgorithm, TestRng, TestRunner};
use std::collections::{BTreeMap, VecDeque};
use std::process::Command;
use telemetry_frame::{self as tf, FrameError, Kind};

// ANCHOR: round_trip
proptest! {
    /// Любые данные проходят кодирование и разбор без изменений.
    #[test]
    #[cfg_attr(miri, ignore = "proptest ищет файл регрессий, а getcwd в изоляции Miri нет")]
    fn slip_round_trip(data in vec(any::<u8>(), 0..64)) {
        prop_assert_eq!(slip::decode(&slip::encode(&data)), Ok(data));
    }
}
// ANCHOR_END: round_trip

// ANCHOR: v1
proptest! {
    #[test]
    #[ignore = "падает намеренно: отчёт сверяет v1_report"]
    fn slip_v1_round_trip(data in vec(any::<u8>(), 0..64)) {
        prop_assert_eq!(slip::decode(&slip::encode_v1(&data)), Ok(data));
    }
}
// ANCHOR_END: v1

/// Запустить свой же бинарник с аргументами libtest и окружением.
fn rerun(args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args(args)
        .envs(env.iter().copied())
        .env("RUST_BACKTRACE", "0")
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code(), text)
}

/// Отчёт proptest об ошибке v1 при фиксированном зерне: уменьшенный
/// пример — один байт ESC.
#[test]
#[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
fn v1_report() {
    let (code, text) = rerun(
        &["--ignored", "--exact", "slip_v1_round_trip"],
        &[("PROPTEST_RNG_SEED", "31"), ("PROPTEST_DISABLE_FAILURE_PERSISTENCE", "1")],
    );
    println!("{text}");
    assert_eq!(code, Some(101));
    assert!(text.contains("minimal failing input: data = [\n    219,\n]"), "{text}");
    assert!(text.contains("Test failed: assertion failed: `(left == right)`"), "{text}");
    assert!(text.contains("left: `Err(BadEscape { at: 1, byte: 192 })`"), "{text}");
}

// ANCHOR: runner
/// То же свойство через `TestRunner` напрямую: зерно задано, файл
/// регрессий пишется во временный каталог, результат — значение.
fn run_v1(persist: Option<&str>) -> (Vec<u8>, String) {
    let mut config = Config {
        failure_persistence: persist
            .map(|p| Box::new(FileFailurePersistence::Direct(p.to_owned().leak())) as _),
        ..Config::default()
    };
    config.source_file = Some("tests/props.rs");
    let rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
    let mut runner = TestRunner::new_with_rng(config, rng);
    let result = runner.run(&vec(any::<u8>(), 0..64), |data| {
        prop_assert_eq!(slip::decode(&slip::encode_v1(&data)), Ok(data));
        Ok(())
    });
    match result {
        Err(proptest::test_runner::TestError::Fail(why, data)) => (data, why.to_string()),
        other => panic!("ожидалась ошибка свойства, получено {other:?}"),
    }
}
// ANCHOR_END: runner

#[test]
#[cfg_attr(miri, ignore = "сотни случаев с уменьшением под Miri слишком долго")]
fn v1_shrinks_to_single_esc() {
    let (data, why) = run_v1(None);
    assert_eq!(data, [slip::ESC]);
    assert!(why.contains("Err(BadEscape { at: 1, byte: 192 })"), "{why}");
}

#[test]
#[cfg_attr(miri, ignore = "файлы и сотни случаев под Miri слишком долго")]
fn regression_file() {
    let path = format!("{}/ch31-regressions.txt", env!("CARGO_TARGET_TMPDIR"));
    let _ = std::fs::remove_file(&path);
    let (first, _) = run_v1(Some(&path));
    let saved = std::fs::read_to_string(&path).unwrap();
    println!("{saved}");
    assert!(saved.starts_with("# Seeds for failure cases proptest has generated in the past."));
    let cc: Vec<&str> = saved.lines().filter(|l| l.starts_with("cc ")).collect();
    assert_eq!(cc.len(), 1, "{saved}");
    assert!(cc[0].ends_with("# shrinks to [219]"), "{saved}");
    // Второй прогон начинает с сохранённого случая и падает на нём же.
    let (second, _) = run_v1(Some(&path));
    assert_eq!(first, second);
}

// ANCHOR: frame_strategies
fn kind() -> impl Strategy<Value = Kind> {
    prop_oneof![Just(Kind::Measurement), Just(Kind::Event), Just(Kind::Ack)]
}

/// Как испортить кадр. Где именно — `Index`: доля от длины, которая
/// станет известна только после кодирования.
#[derive(Clone, Debug)]
enum Damage {
    None,
    FlipBit(Index, u8),
    Truncate(Index),
    Insert(Index, u8),
}

fn damage() -> impl Strategy<Value = Damage> {
    prop_oneof![
        1 => Just(Damage::None),
        2 => (any::<Index>(), 0u8..8).prop_map(|(i, b)| Damage::FlipBit(i, b)),
        1 => any::<Index>().prop_map(Damage::Truncate),
        1 => (any::<Index>(), any::<u8>()).prop_map(|(i, b)| Damage::Insert(i, b)),
    ]
}

/// Настоящий кадр, а потом, может быть, одна порча.
fn damaged_frame() -> impl Strategy<Value = Vec<u8>> {
    (kind(), any::<u8>(), vec(any::<u8>(), 0..=32), damage()).prop_map(|(k, seq, p, d)| {
        let mut buf = vec![0; tf::HEADER_LEN + p.len()];
        let n = tf::encode(k, seq, &p, &mut buf).unwrap();
        match d {
            Damage::None => {}
            Damage::FlipBit(i, bit) => buf[i.index(n)] ^= 1 << bit,
            Damage::Truncate(i) => buf.truncate(i.index(n)),
            Damage::Insert(i, b) => buf.insert(i.index(n + 1), b),
        }
        buf
    })
}
// ANCHOR_END: frame_strategies

// ANCHOR: frame_props
proptest! {
    #[test]
    #[cfg_attr(miri, ignore = "proptest ищет файл регрессий, а getcwd в изоляции Miri нет")]
    fn frame_round_trip(kind in kind(), seq: u8, payload in vec(any::<u8>(), 0..=tf::MAX_PAYLOAD)) {
        let mut buf = [0; tf::HEADER_LEN + tf::MAX_PAYLOAD];
        let n = tf::encode(kind, seq, &payload, &mut buf).unwrap();
        let f = tf::decode(&buf[..n]).unwrap();
        prop_assert_eq!((f.header.kind, f.header.seq), (kind, seq));
        prop_assert_eq!(f.payload, &payload[..]);
    }

    /// Табличная CRC — та же функция, что побитовая: модель — побитовая.
    #[test]
    #[cfg_attr(miri, ignore = "proptest ищет файл регрессий, а getcwd в изоляции Miri нет")]
    fn crc_table_matches_bitwise(crc: u16, data in vec(any::<u8>(), 0..32)) {
        prop_assert_eq!(tf::crc16_update_table(crc, &data), tf::crc16_update(crc, &data));
    }

    /// На испорченном кадре decode не паникует, а принятый кадр
    /// кодируется обратно в те же байты.
    #[test]
    #[cfg_attr(miri, ignore = "proptest ищет файл регрессий, а getcwd в изоляции Miri нет")]
    fn decode_survives_damage(buf in damaged_frame()) {
        if let Ok(f) = tf::decode(&buf) {
            let mut out = [0; tf::HEADER_LEN + tf::MAX_PAYLOAD];
            let n = tf::encode(f.header.kind, f.header.seq, f.payload, &mut out).unwrap();
            prop_assert_eq!(&out[..n], &buf[..n]);
        }
    }
}
// ANCHOR_END: frame_props

/// Куда доходит `decode` на входах стратегии: имя ошибки или «Ok».
fn outcome(buf: &[u8]) -> &'static str {
    match tf::decode(buf) {
        Ok(_) => "Ok",
        Err(FrameError::Short { .. }) => "Short",
        Err(FrameError::BadMagic(_)) => "BadMagic",
        Err(FrameError::BadVersion(_)) => "BadVersion",
        Err(FrameError::BadKind(_)) => "BadKind",
        Err(FrameError::TooLong(_)) => "TooLong",
        Err(FrameError::BadCrc { .. }) => "BadCrc",
        Err(e) => panic!("decode не выдаёт {e:?}"),
    }
}

fn histogram<S: Strategy<Value = Vec<u8>>>(s: S, n: usize) -> BTreeMap<&'static str, usize> {
    let rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
    let mut runner = TestRunner::new_with_rng(Config::default(), rng);
    let mut h = BTreeMap::new();
    for _ in 0..n {
        let buf = s.new_tree(&mut runner).unwrap().current();
        *h.entry(outcome(&buf)).or_default() += 1;
    }
    h
}

// ANCHOR: histogram
#[test]
#[cfg_attr(miri, ignore = "тысячи случаев под Miri слишком долго")]
fn strategies_reach_different_depths() {
    let random = histogram(vec(any::<u8>(), 0..300), 1000);
    let damaged = histogram(damaged_frame(), 1000);
    println!("случайные байты: {random:?}");
    println!("испорченные кадры: {damaged:?}");
    // Случайный буфер почти всегда отвергнут первой же проверкой, а
    // остальные — короткие.
    assert!(random["BadMagic"] > 950, "{random:?}");
    assert!(!random.contains_key("Ok") && !random.contains_key("BadCrc"));
    // Кадр с одной порчей доходит до CRC и до конца разбора.
    assert!(damaged["Ok"] > 150 && damaged["BadCrc"] > 150, "{damaged:?}");
}
// ANCHOR_END: histogram

// ANCHOR: model
#[derive(Clone, Debug)]
enum Op {
    Push(i32),
    Mean,
    Latest(Index),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => any::<i32>().prop_map(Op::Push),
        1 => Just(Op::Mean),
        1 => any::<Index>().prop_map(Op::Latest),
    ]
}

proptest! {
    /// Окно ведёт себя как модель: очередь из последних N измерений.
    #[test]
    #[cfg_attr(miri, ignore = "proptest ищет файл регрессий, а getcwd в изоляции Miri нет")]
    fn window_matches_model(ops in vec(op(), 0..100)) {
        const N: usize = 5;
        let mut w = Window::<N>::new();
        let mut model = VecDeque::new();
        for op in ops {
            match op {
                Op::Push(v) => {
                    w.push(v);
                    model.push_back(v);
                    if model.len() > N {
                        model.pop_front();
                    }
                }
                Op::Mean => {
                    let sum: i64 = model.iter().map(|&v| i64::from(v)).sum();
                    let want = (!model.is_empty()).then(|| (sum / model.len() as i64) as i32);
                    prop_assert_eq!(w.mean(), want);
                }
                Op::Latest(i) if !model.is_empty() => {
                    let age = i.index(model.len());
                    prop_assert_eq!(w.latest(age), model[model.len() - 1 - age]);
                }
                Op::Latest(_) => {}
            }
            prop_assert_eq!(w.len(), model.len());
        }
    }
}
// ANCHOR_END: model
