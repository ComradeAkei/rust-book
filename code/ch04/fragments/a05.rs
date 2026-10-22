pub fn f(mut x: u32) -> u32 {
    let r = &mut x;
    x = 1;
    *r += 1;
    x
}
