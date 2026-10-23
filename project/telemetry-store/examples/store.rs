//! Шаг П15 вживую: наполняем хранилище, потом проверяем, что
//! установившийся приём не трогает кучу. Аллокатор-счётчик стоит на весь
//! процесс, а процесс однопоточный, поэтому числа детерминированы.
//!
//! ```text
//! cargo run --example store -p telemetry-store
//! ```

use core::num::NonZeroU16;
use telemetry_frame::{Kind, Measurement, encode};
use telemetry_store::{Counted, Store, alloc::counts};

#[global_allocator]
static COUNTED: Counted = Counted;

const SENSORS: u16 = 4;
const CAP: usize = 64;

/// Поток из `rounds` измерений на каждый из `SENSORS` датчиков.
fn stream(rounds: u32, t0: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = [0u8; 32];
    let mut seq = 0u8;
    for r in 0..rounds {
        for s in 1..=SENSORS {
            let m = Measurement {
                sensor: NonZeroU16::new(s).unwrap(),
                timestamp_ms: t0 + r * 100,
                value: (s as i32) * 10 + r as i32,
            };
            let n = encode(Kind::Measurement, seq, &m.to_bytes(), &mut buf).unwrap();
            out.extend_from_slice(&buf[..n]);
            seq = seq.wrapping_add(1);
        }
    }
    out
}

fn main() {
    let mut store = Store::new(CAP);

    // Разогрев: заполняем кольца доверху. Здесь куча и выделяется —
    // по буферу на датчик и рост таблицы.
    let warm = stream(CAP as u32, 1_000);
    let before_warm = counts();
    store.ingest(&warm);
    let after_warm = counts();
    let s = store.summary();
    println!(
        "разогрев: {} датчика, в кольце до {} измерений — всего {} точек; за разогрев {} выделений",
        s.sensors,
        store.capacity(),
        s.points,
        after_warm.allocs - before_warm.allocs
    );

    // Установившийся режим: те же датчики, кольца полны. Буфер потока
    // построен заранее, до снимка счётчиков.
    let steady = stream(16, 1_000_000);
    let before = counts();
    let rep = store.ingest(&steady);
    let after = counts();
    assert_eq!(rep.stored, 16 * SENSORS as usize);
    println!(
        "установившийся приём той же порции: выделений {}, освобождений {}, занято {}",
        after.allocs - before.allocs,
        after.deallocs - before.deallocs,
        if after.live == before.live {
            "не изменилось"
        } else {
            "изменилось"
        }
    );

    // Запрос по времени: самые свежие измерения датчика 2 из разных окон.
    let s2 = NonZeroU16::new(2).unwrap();
    let last = store.latest(s2).unwrap();
    let from = last.t_ms - 500;
    let n = store.range(s2, from, last.t_ms).count();
    println!(
        "датчики по возрастанию: {:?}",
        store.sensors_sorted().iter().map(|s| s.get()).collect::<Vec<_>>()
    );
    println!(
        "датчик 2: {} точек за последние 500 мс, последняя = {} в {} мс",
        n, last.value, last.t_ms
    );
}
