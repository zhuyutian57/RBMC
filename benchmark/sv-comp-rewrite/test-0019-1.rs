use std::{alloc::{alloc, dealloc, Layout}, ptr};

struct TData {
    lo : *mut u8,
    hi : *mut u8,
}

fn alloc_data(pdata : &mut TData) {
    pdata.lo = unsafe { alloc(Layout::new::<u16>()) };
    pdata.hi = unsafe { alloc(Layout::new::<[u8; 3]>()) };
}

fn free_data(data : TData) {
    let lo = data.lo;
    let hi = data.hi;
    
    if lo == hi { return; }

    unsafe {
        dealloc(lo, Layout::new::<u16>());
        dealloc(hi, Layout::new::<[u8; 3]>());
    }
}

fn main() {
    let mut data = TData {
        lo : ptr::null_mut() as *mut u8,
        hi : ptr::null_mut() as *mut u8,
    };
    alloc_data(&mut data);
    free_data(data);
}

// safe