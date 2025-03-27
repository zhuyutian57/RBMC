use std::{alloc::{alloc, dealloc, Layout}, ptr};

fn main() {
    struct T {
        next : *const T,
    };

    let mut x = ptr::null::<T>();
    let mut y = ptr::null::<T>();
    
    let layout = Layout::new::<T>();

    y = unsafe { alloc(layout) as *const T };
    let adressY = y as usize;
    
    unsafe { dealloc(y as *mut u8, layout); }

    x = unsafe { alloc(layout) as *const T };
    let adressX = x as usize;

    if adressX == adressY {
        // if the second malloc returns the same value as the first, I should get here
        unsafe { dealloc(x as *mut u8, layout); }
    }

    unsafe { dealloc(x as *mut u8, layout); } // invalid-free for the reused memory mode
}