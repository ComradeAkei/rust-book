pub fn f(p: &mut (u8, u8)) {
    let x = &mut p.0;
    let y = &mut p.1;
    *x += *y;
}
