use std::{alloc::{alloc, dealloc, Layout}, ptr};


unsafe fn create_ref_from_raw(p: *mut i32) -> &'static mut i32 { &mut *p }

fn main() {
  let p = unsafe { alloc(Layout::new::<i32>()) as *mut i32 };
  let bp = unsafe { Box::from_raw(p) };
  // create aliasing reference that violate borrow checker
  let r1 = unsafe { create_ref_from_raw(p) };
  let r2 = unsafe { create_ref_from_raw(p) };
  // borrow checker fail to check that r1 and r2 violate rules
  if ptr::eq(r1, r2) {
    unsafe { dealloc(p as *mut u8, Layout::new::<i32>()) };
  }
  unsafe { dealloc(p as *mut u8, Layout::new::<i32>()) };
}