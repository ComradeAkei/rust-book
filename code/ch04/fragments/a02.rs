pub fn f(a: &mut [u8; 4]) {
    let x = &mut a[0];
    let y = &mut a[1];
    *x += *y;
}
