use std::{alloc::Layout, ptr};

struct Node(i32, i32);

const N : usize = 128;
static mut A : Node = Node(0, 0);
static mut B : i32 = 12;

unsafe fn foo() {
  B = 301;
  A = Node(1, 2);
}

fn main() {
  unsafe {
    // println!("{a:p} - {b:p} - {c:p}");
    // println!("{e:p} - {d:p} - {:p}", &n);
    let x = N * N;
    B = 101;
    let y = B as usize;
  }
}