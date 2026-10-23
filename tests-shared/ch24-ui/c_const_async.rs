// #[timed] не ставится на const fn и async fn.
use std::sync::atomic::AtomicU32;

static STATS: AtomicU32 = AtomicU32::new(0);

#[tasks_macros::timed(STATS)]
const fn table() -> u32 {
    42
}

#[tasks_macros::timed(STATS)]
async fn fetch() -> u32 {
    7
}

fn main() {}
