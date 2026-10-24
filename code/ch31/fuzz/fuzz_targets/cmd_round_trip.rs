// ANCHOR: all
#![no_main]

use ch31::cmd::{Command, parse};
use libfuzzer_sys::fuzz_target;

// Вход — уже команда: libfuzzer-sys строит её из байтов через Arbitrary.
fuzz_target!(|cmd: Command| {
    let mut out = [0; 39];
    let n = cmd.encode(&mut out);
    assert_eq!(parse(&out[..n]), Ok(cmd));
});
// ANCHOR_END: all
