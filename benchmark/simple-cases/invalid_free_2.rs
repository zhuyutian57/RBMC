use std::alloc::{alloc, Layout};

// #[kani::proof]
fn main() {
    let layout = Layout::new::<Box<i32>>();
    let mut x = unsafe { alloc(layout) as *mut Box<i32> };
    // The dereferenceof 'x' calls `drop` of `Box`
    unsafe { *x = Box::new(123); } // invalid-free
}