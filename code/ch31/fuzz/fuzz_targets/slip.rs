// ANCHOR: all
#![no_main]

use ch31::slip;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|wire: &[u8]| {
    // У каждого байта данных одна запись на проводе, поэтому принятый
    // кадр — ровно кодирование своих данных.
    if let Ok(data) = slip::decode(wire) {
        assert_eq!(slip::encode(&data), wire);
    }
});
// ANCHOR_END: all
