pub fn f(v: &mut Vec<u32>) {
    let n = &mut v[0];
    *n += v.len() as u32;
}
