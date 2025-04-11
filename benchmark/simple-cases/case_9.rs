use std::alloc::*;

extern crate rbmc;

fn main() {
    let ptr = unsafe { alloc(Layout::new::<i32>()) as *mut i32 };
    let b = 
        if rbmc::nondet::<bool>() {
            unsafe { Box::from_raw(ptr) }
        } else {
            Box::new(123)
        };
    unsafe {
        dealloc(ptr as *mut u8, Layout::new::<i32>());
    }
}