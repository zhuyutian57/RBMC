
struct Node {
  x: i32,
  y: i32,
}

fn main() {
  let mut n = Node { x: 10, y: 100 };
  let x = &n.x;
  let y = &n.y;
  let z = *x + *y;
  if z == 100 {
    n.x = 101;
  } else {
    n.y = 101;
  }
}