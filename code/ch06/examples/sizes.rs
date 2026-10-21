//! Печатает таблицу размеров и выравниваний для типов главы 6.
//!
//! ```text
//! cargo run -p ch06 --example sizes
//! ```
//! Та же таблица для Cortex-M3 печатается прошивкой `firmware/ch06-uart`.

use ch06::enums::{Message, MessageC, MessageU8};
use ch06::layout::{Record, RecordC, Sample, SampleC, SampleSorted};
use ch06::niche::{Reading, SensorId};
use ch06::packed::WireSample;
use core::mem::{align_of, size_of};
use core::num::NonZeroU32;

macro_rules! row {
    ($($t:ty),* $(,)?) => {
        $( println!("{:<28} {:>5} {:>6}", stringify!($t), size_of::<$t>(), align_of::<$t>()); )*
    };
}

fn main() {
    println!("{:<28} {:>5} {:>6}", "тип", "size", "align");
    row!(
        Sample,
        SampleC,
        SampleSorted,
        WireSample,
        Record,
        RecordC,
        Message,
        MessageC,
        MessageU8,
        Option<SensorId>,
        Option<Reading>,
        Option<NonZeroU32>,
        Option<u32>,
        Option<&u8>,
        Option<char>,
        usize,
        &[u8],
    );
    println!("offsets Sample: {:?}", ch06::layout::sample_offsets());
}
