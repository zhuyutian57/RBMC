use std::alloc::*;

extern crate mirv;

fn main() {
    let ptr = unsafe { alloc(Layout::new::<i32>()) as *mut i32 };
    let b = 
        if mirv::nondet::<bool>() {
            unsafe { Box::from_raw(ptr) }
        } else {
            Box::new(123)
        };
    unsafe {
        dealloc(ptr as *mut u8, Layout::new::<i32>());
    }
}