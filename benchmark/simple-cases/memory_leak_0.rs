
fn main() {
    let mut x = Box::new(0);
    let mut p = Box::into_raw(x);
    // memory-leak
}