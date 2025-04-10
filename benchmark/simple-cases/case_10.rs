
struct Node {
  x: i32,
  y: i32,
}

pub enum Te {
  A,
  B(i32),
  C{ x: i32, y: i32 },
}

fn create() -> Option<Node> {
  Some(Node { x: 10, y: 100 })
}

fn main() {
  let t1 = Te::A;
  let t2 = Te::B(0);
  let mut t3 = Te::C{x: 55, y: 66};
  let mut n = create();
  let mut y1 = 0;
  if let Some(nod) = n {
    y1 = nod.y;
  } else {
    y1 = 1;
  }
  let mut x1 = y1;
  if let Te::C{ x, y } = &mut t3 {
    *x = x1;
    *y = y1;
  }
}