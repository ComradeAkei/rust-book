pub fn f(v: &mut Vec<u32>) {
    for x in v.iter() {
        if *x == 0 {
            v.push(1);
        }
    }
}
