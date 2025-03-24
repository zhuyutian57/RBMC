use std::alloc::{alloc, dealloc, Layout};

// #[kani::proof]
fn main() {
    let layout = Layout::new::<i32>();
    // The memory allocated by `alloc` should be
    // manually deallocated by user
    let p1 = unsafe { alloc(layout) } as *mut i32;
    // `p2` take the ownership of memory of `p1` 
    let p2 = unsafe { Box::from_raw(p1) }; 
    unsafe { dealloc(p1 as *mut u8, layout); }
} // invalid-free, double free