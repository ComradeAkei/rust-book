pub fn f(v: &mut Vec<u32>) {
    if let Some(last) = v.last() {
        v.push(*last);
    }
}
