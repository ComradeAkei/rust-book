//! Раздел 31.1: интеграционный тест — отдельный крейт, который видит
//! только публичный API `ch31`.

mod common;

use ch31::basics::Window;
use common::window_of;

// ANCHOR: integration
#[test]
fn mean_over_last_n() {
    let w = window_of::<3>(&[1, 2, 3, 4, 5, 6]);
    assert_eq!(w.len(), 3);
    assert_eq!(w.mean(), Some(5));
    assert_eq!((w.latest(0), w.latest(2)), (6, 4));
}

#[test]
fn display_round_trips() {
    let w = window_of::<4>(&[-1, 0, 7]);
    let back: Window<4> = w.to_string().parse().unwrap();
    assert_eq!(back.to_string(), "-1,0,7");
    assert_eq!(back.mean(), w.mean());
}
// ANCHOR_END: integration

#[test]
fn empty_window() {
    let w: Window<5> = "".parse().unwrap();
    assert!(w.is_empty());
    assert_eq!(w.mean(), None);
    assert_eq!(w.to_string(), "");
}
