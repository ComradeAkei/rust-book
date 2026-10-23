//! Раздел 15.6: что просят у распределителя обычные строчки кода.
//! `Trace` из библиотеки стоит на месте системного распределителя и
//! записывает обращения одного потока.
//!
//! Запуск: `cargo run --release -p ch15 --example trace`.
use ch15::alloc_api::{Event, Trace};
use std::collections::{BTreeMap, HashMap};
use std::hint::black_box;

// ANCHOR: install
#[global_allocator]
static TRACE: Trace = Trace::new();
// ANCHOR_END: install

fn show(what: &str, events: Option<Vec<Event>>) {
    let events = events.expect("больше 256 обращений");
    let text: Vec<String> = events
        .iter()
        .map(|e| match *e {
            Event::Alloc { size, align } => format!("alloc {size}/{align}"),
            Event::Zeroed { size, align } => format!("alloc_zeroed {size}/{align}"),
            Event::Realloc { size, new_size, align } => {
                format!("realloc {size}→{new_size}/{align}")
            }
            Event::Dealloc { size, align } => format!("dealloc {size}/{align}"),
        })
        .collect();
    let text = if text.is_empty() { "ничего".to_string() } else { text.join(", ") };
    println!("{what:32} {text}");
}

fn main() {
    // ANCHOR: cases
    show("vec![1u32, 2, 3]", TRACE.record(|| black_box(vec![1u32, 2, 3])).1);
    show("vec![0u64; 1000]", TRACE.record(|| black_box(vec![0u64; 1000])).1);
    show(
        "Vec<u32>, 9 раз push",
        TRACE
            .record(|| {
                let mut v = Vec::new();
                for i in 0..9u32 {
                    v.push(black_box(i));
                }
                v
            })
            .1,
    );
    show("String::from(\"датчик\")", TRACE.record(|| black_box(String::from("датчик"))).1);
    show(
        "format!(\"{}={}\", s, v)",
        TRACE.record(|| format!("{}={}", black_box("adc0"), black_box(1250))).1,
    );
    show(
        "Vec<()>, 1000 раз push",
        TRACE
            .record(|| {
                let mut v = Vec::new();
                for _ in 0..1000 {
                    v.push(());
                }
                v
            })
            .1,
    );
    show("HashMap::new()", TRACE.record(HashMap::<u32, u32>::new).1);
    show("HashMap<u32, u32>, 1 insert", TRACE.record(|| HashMap::from([(1u32, 1u32)])).1);
    show(
        "BTreeMap<u32, u32>, 12 insert",
        TRACE.record(|| (0..12u32).map(|k| (k, k)).collect::<BTreeMap<_, _>>()).1,
    );
    show(
        "into_boxed_slice при len < cap",
        TRACE
            .record(|| {
                let mut v = Vec::<u8>::with_capacity(100);
                v.extend_from_slice(b"adc0");
                v.into_boxed_slice()
            })
            .1,
    );
    show("drop(Vec<u32> на 16)", {
        let v: Vec<u32> = (0..16).collect();
        TRACE.record(|| drop(v)).1
    });
    // ANCHOR_END: cases
}
