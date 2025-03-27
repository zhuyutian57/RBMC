
fn index() -> usize { 100 }

fn main() {
  let mut array = [0; 5];
  // The assert is builtin in Rust. No need to generate bound check.
  array[1] = 1;
  array[2] = 3;
  array[index()] = 10312;
}