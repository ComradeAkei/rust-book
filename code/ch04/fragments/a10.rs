pub fn f(v: &mut Vec<u32>) {
    let a = v.last_mut().unwrap();
    let b = v.first_mut().unwrap();
    core::mem::swap(a, b);
}
