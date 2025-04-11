
fn main() {
    let mut v1 = Vec::new();
    v1.push(1);
    v1.push(12);
    let x = v1[0] == v1[1];
    if !x { v1.pop(); }
    let y = v1.pop();
    let z = match y {
        Some(i) => i,
        None => 0,
    };
}