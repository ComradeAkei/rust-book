// Общие тесты шага П15: подключаются через include! в крейты заготовки и
// решения после строки `use <крейт> as sut;`. Аллокатор-счётчик крейта
// стоит на весь тестовый процесс. Тесты идут параллельно, в разных
// потоках, а счётчик общий на процесс, поэтому ровных равенств по числу
// выделений здесь нет: проверяется только то, что устойчиво к соседним
// потокам (счётчик растёт при заведомом выделении). Точное «в
// установившемся режиме выделений ноль» проверяет отдельный тест решения
// через запуск примера в своём процессе.

use core::num::NonZeroU16;
use sut::alloc::counts;
use sut::{Ingested, Store};
use telemetry_frame::{Kind, Measurement, encode};

#[global_allocator]
static COUNTED: sut::Counted = sut::Counted;

fn nz(n: u16) -> NonZeroU16 {
    NonZeroU16::new(n).unwrap()
}

/// Дописать в `out` кадр-измерение.
fn meas(out: &mut Vec<u8>, sensor: u16, t_ms: u32, value: i32, seq: u8) {
    let m = Measurement { sensor: nz(sensor), timestamp_ms: t_ms, value };
    let mut buf = [0u8; 32];
    let n = encode(Kind::Measurement, seq, &m.to_bytes(), &mut buf).unwrap();
    out.extend_from_slice(&buf[..n]);
}

/// Дописать подтверждение без нагрузки.
fn ack(out: &mut Vec<u8>, seq: u8) {
    let mut buf = [0u8; 8];
    let n = encode(Kind::Ack, seq, &[], &mut buf).unwrap();
    out.extend_from_slice(&buf[..n]);
}

#[test]
fn p15_ingest_and_summary() {
    let mut stream = Vec::new();
    stream.push(0x00); // мусор перед первым кадром
    meas(&mut stream, 7, 1000, -42, 0);
    meas(&mut stream, 2, 1100, 5, 1);
    ack(&mut stream, 2);
    meas(&mut stream, 7, 1200, -40, 3);

    let mut store = Store::new(8);
    let rep = store.ingest(&stream);
    assert_eq!(
        rep,
        Ingested { frames: [3, 0, 1], stored: 3, bad_payload: 0, errors: 1 },
        "мусорный байт — одна ошибка, три измерения и одно подтверждение"
    );

    let s = store.summary();
    assert_eq!((s.sensors, s.points), (2, 3));
    assert_eq!((s.oldest_ms, s.newest_ms), (Some(1000), Some(1200)));
    assert_eq!(store.len(nz(7)), 2);
    assert_eq!(store.latest(nz(7)).map(|p| p.value), Some(-40));
}

#[test]
fn p15_ring_evicts_oldest() {
    let mut stream = Vec::new();
    for (i, t) in [10u32, 20, 30, 40, 50].into_iter().enumerate() {
        meas(&mut stream, 3, t, t as i32, i as u8);
    }
    let mut store = Store::new(3);
    store.ingest(&stream);
    // Кольцо на 3: три последних, самые старые вытеснены.
    assert_eq!(store.len(nz(3)), 3);
    assert_eq!(store.latest(nz(3)).map(|p| p.t_ms), Some(50));
    let got: Vec<u32> = store.range(nz(3), 0, 1000).map(|p| p.t_ms).collect();
    assert_eq!(got, [30, 40, 50]);
}

#[test]
fn p15_range_inclusive_bounds() {
    let mut stream = Vec::new();
    for (i, t) in [10u32, 20, 30, 40].into_iter().enumerate() {
        meas(&mut stream, 1, t, 0, i as u8);
    }
    let mut store = Store::new(16);
    store.ingest(&stream);
    let at = |from, to| store.range(nz(1), from, to).map(|p| p.t_ms).collect::<Vec<_>>();
    assert_eq!(at(20, 30), [20, 30]); // границы включаются
    assert_eq!(at(21, 29), [] as [u32; 0]); // между точками — пусто
    assert_eq!(at(0, 1000), [10, 20, 30, 40]);
    assert_eq!(at(35, 100), [40]);
    // Незнакомый датчик — пустой итератор, без паники.
    assert_eq!(store.range(nz(999), 0, u32::MAX).count(), 0);
}

#[test]
fn p15_sensors_sorted() {
    let mut stream = Vec::new();
    for (i, id) in [7u16, 2, 9, 2].into_iter().enumerate() {
        meas(&mut stream, id, i as u32, 0, i as u8);
    }
    let mut store = Store::new(8);
    store.ingest(&stream);
    assert_eq!(store.sensors_sorted(), [nz(2), nz(7), nz(9)]);
    assert_eq!(store.sensor_count(), 3);
}

#[test]
fn p15_counter_sees_allocations() {
    use std::hint::black_box;
    // Нижние границы: соседние потоки могут только добавить выделений, но
    // не убрать, поэтому строгий рост счётчиков устойчив к параллели.
    let before = counts();
    let big = black_box(Vec::<u64>::with_capacity(4096));
    let during = counts();
    assert!(during.allocs > before.allocs, "{before:?} -> {during:?}");
    assert!(during.peak >= before.peak);
    let d0 = counts().deallocs;
    drop(big);
    assert!(counts().deallocs > d0);
}
