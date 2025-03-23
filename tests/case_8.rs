use std::alloc::{alloc, Layout};


// #[kani::proof]
fn main() {
  let raw = unsafe { alloc(Layout::new::<(i32, i32)>()) as *mut (i32, i32) };
  let b = unsafe { Box::from_raw(raw) };
  let t = *b;
  let bb = unsafe { Box::from_raw(raw) };
  let raw_agin = Box::into_raw(bb);
}