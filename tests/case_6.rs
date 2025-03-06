
fn index() -> usize {
  100
}

fn main() {
  let mut array = [0; 5];
  array[index()] = 1;
}