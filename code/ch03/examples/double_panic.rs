//! Раздел 3.6: паника в деструкторе во время раскрутки стека.
//!
//! ```text
//! cargo run -p ch03 --example double_panic
//! ```
//!
//! Первая паника начинает раскрутку, раскрутка вызывает деструктор, а тот
//! паникует снова. Две раскрутки одновременно не бывает: процесс
//! завершается через abort, и остальные деструкторы уже не выполнятся.

// ANCHOR: flaky
struct Flaky(&'static str);

impl Drop for Flaky {
    fn drop(&mut self) {
        eprintln!("drop {}", self.0);
        if self.0 == "b" {
            panic!("деструктор тоже паникует");
        }
    }
}

fn main() {
    let _a = Flaky("a");
    let _b = Flaky("b");
    let _c = Flaky("c");
    panic!("первая паника");
}
// ANCHOR_END: flaky
