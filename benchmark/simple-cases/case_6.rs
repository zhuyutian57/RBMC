
fn index() -> usize {
  100
}

// #[kani::proof]
fn main() {
  let mut array = [0; 5];
  array[index()] = 1;
}