// Раздел 23.3: swap! после раскрытия — с номерами контекстов гигиены.
// Снимок рядом, в swap.out.rs:
// RUSTC_BOOTSTRAP=1 rustc --edition 2024 --crate-type lib -Zunpretty=expanded,hygiene swap.rs
macro_rules! swap {
    ($a:ident, $b:ident) => {
        let tmp = $a;
        $a = $b;
        $b = tmp;
    };
}

pub fn call(mut x: i32, mut tmp: i32) -> (i32, i32) {
    swap!(x, tmp);
    (x, tmp)
}
