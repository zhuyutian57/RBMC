use std::alloc::*;

struct Node { x : i32, y: u64 }

fn f(n : &mut Node) {
  *n = Node { x: 100, y: 1213, };
}

fn g(x: i32) -> i32 {
  1910 * x
}

fn main() {
  let mut n1 = Node { x : -12, y : 100 };
  let mut n2 = Node { x : 13, y : 100 };
  let mut boxn = Box::new(Node { x : 14, y : 99 });
  f(&mut n1); f(&mut n2);
  let pn =
    if n1.x > n2.x {
        &mut n1 as *mut Node
    } else if n1.x < n2.x {
        unsafe { alloc(Layout::new::<Node>()) as *mut Node }
    } else {
        &mut *boxn as *mut Node
    };
  let bn = if n1.x == n2.x { &mut n1 } else { &mut n2 };
  unsafe { *pn = Node { x : 190, y : 54 }; }
  let t = &mut *boxn;
  // for i in 0..10 {
  //   let x = 10;
  //   t.x *= x;
  // }
  t.x = g(t.x);
}