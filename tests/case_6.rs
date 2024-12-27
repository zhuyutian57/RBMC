use std::ops::{Deref, DerefMut};

fn f(x: &mut i32) {}

fn main() {
  let mut local = 0;
  let mut x = &mut local;
  // let y = &mut *x.deref_mut();
  f(x);
  *x = 101;
  local = 10;
}