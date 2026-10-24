// ANCHOR: all
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Принятая команда кодируется обратно в те же байты.
    if let Ok(cmd) = ch31::cmd::parse(data) {
        let mut out = [0; 39];
        let n = cmd.encode(&mut out);
        assert_eq!(&out[..n], &data[..n]);
    }
});
// ANCHOR_END: all
