pub fn f(buf: &mut [u8; 8]) -> u8 {
    let head = &buf[..4];
    *buf = [0; 8];
    head[0]
}
