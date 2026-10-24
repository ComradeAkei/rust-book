//! Задача 31.D, часть 1: тесты хранилища проекта (`project/telemetry-store`),
//! которые убивают мутантов, выживших после
//! `cargo mutants -p telemetry-store`: таких шесть. Проверка — общий тест
//! `d_store_mutants_killed`, он запускает
//!
//! ```text
//! cargo mutants -p telemetry-store --test-package telemetry-store,ch31-solutions \
//!     --baseline skip -- --test store --test d_store
//! ```

use telemetry_frame::{Kind, encode};
use telemetry_store::alloc::counts;
use telemetry_store::{Counted, Store};

#[global_allocator]
static COUNTED: Counted = Counted;

// ANCHOR: capacity
/// Мутанты `capacity` → `0` и → `1`: ёмкость тесты проекта не читали.
#[test]
fn capacity_is_what_was_asked() {
    assert_eq!(Store::new(5).capacity(), 5);
    assert_eq!(Store::new(0).capacity(), 1);
}
// ANCHOR_END: capacity

// ANCHOR: bad_payload
/// Мутанты `rep.bad_payload += 1` → `-=` и → `*=`: измерений с неверной
/// нагрузкой в тестах проекта не было. `-=` паникует на переполнении,
/// `*=` оставляет ноль.
#[test]
fn bad_payloads_are_counted() {
    let mut stream = Vec::new();
    let mut buf = [0u8; 32];
    for seq in 0..2 {
        let n = encode(Kind::Measurement, seq, &[1, 2, 3], &mut buf).unwrap();
        stream.extend_from_slice(&buf[..n]);
    }
    let rep = Store::new(4).ingest(&stream);
    assert_eq!((rep.frames[0], rep.stored, rep.bad_payload), (2, 0, 2));
}
// ANCHOR_END: bad_payload

// ANCHOR: live
/// Мутанты `bump_live` → `()` и `before + delta` → `before * delta`:
/// занятое и пик тесты проекта не сверяли. Пик только растёт, и у мутанта
/// он давно огромен — `before * delta` для любого прежнего выделения.
/// Поэтому блоки большие: держим 16 МиБ и выделяем ещё столько же, тогда
/// мутант даёт пик не меньше 2⁴⁸ и обойти его прежним значением не может.
/// Память только обещана ядром: блоки никто не трогает. Счётчики общие на
/// процесс, а тесты идут в соседних потоках, поэтому замер повторяется,
/// пока между двумя снимками не случится ровно одно выделение — наше;
/// каждая попытка удваивает блок, чтобы пик неудачной не заслонил следующую.
#[test]
fn live_and_peak_follow_allocation() {
    const MIB16: usize = 16 << 20;
    let hold: Vec<u8> = std::hint::black_box(Vec::with_capacity(MIB16));
    for k in 0..6 {
        let size = MIB16 << k;
        let before = counts();
        let v: Vec<u8> = std::hint::black_box(Vec::with_capacity(size));
        let after = counts();
        drop(v);
        if after.allocs != before.allocs + 1 || after.deallocs != before.deallocs {
            continue;
        }
        assert_eq!(after.live, before.live + size);
        assert_eq!(after.peak, before.peak.max(before.live + size));
        drop(hold);
        return;
    }
    panic!("за 6 попыток не удалось снять замер без чужих выделений");
}
// ANCHOR_END: live
