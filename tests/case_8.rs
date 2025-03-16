use std::{alloc::Layout, ptr};

struct Node(i32, i32);

const n : usize = 128;
static mut a : Node = Node(0, 0);
static mut f : i32 = 12;

unsafe fn foo() {
  f = 301;
  a = Node(1, 2);
}

fn main() {
  unsafe {
    // println!("{a:p} - {b:p} - {c:p}");
    // println!("{e:p} - {d:p} - {:p}", &n);
    let x = n * n;
    f = 101;
    let y = f as usize;
  }
}