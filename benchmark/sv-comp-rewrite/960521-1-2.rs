
use std::{alloc::{alloc, dealloc, Layout}, ptr};

// static n : usize = 32768;

static n : usize = 3276;
static mut a : *mut i32 = ptr::null_mut();
static mut b : *mut i32 = ptr::null_mut();

fn foo() {
    let mut i = 0;
    loop {
        unsafe { *a.offset(i as isize) = -1; }
        i += 1;
        if i == n { break; }
    }
    i = 0;
    loop {
        unsafe { *b.offset(i as isize) = -1; }
        i += 1;
        if i == n - 1 { break; }
    }
}

// #[kani::proof]
fn main() {
    let layout = Layout::new::<[i32; n]>();
    unsafe { a =  alloc(layout) as *mut i32 };
    unsafe { b =  alloc(layout) as *mut i32 };
    unsafe {
        *b = 0;
        b = b.add(1);
    }
    foo();
    unsafe {
        if *(b.offset(-1)) != 0 {
            dealloc(a as *mut u8, layout);
            dealloc(b as *mut u8, layout);
        } else {
            dealloc(a as *mut u8, layout);
            dealloc(b.offset(-1) as *mut u8, layout);
        }
    }
}

// safe