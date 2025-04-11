use std::{alloc::{alloc, dealloc, Layout}, ptr};

struct item {
    next : *mut item,
    data : *mut item,
}

fn append(plist : *mut *mut item) {
    let layout = Layout::new::<item>();

    let item = unsafe { alloc(layout) as *mut item };

    unsafe {
        (*item).next = *plist;

        (*item).data =
            if !(*item).next.is_null() {
                (*((*item).next)).data
            } else {
                alloc(layout) as *mut item
            };
        
        *plist = item;
    }
}

extern crate rbmc;

fn main() {
    let mut list : *mut item = ptr::null_mut();

    loop {
        append(&mut list as *mut *mut item);
        
        if rbmc::nondet::<usize>() == 0 { break; }
    }

    if !list.is_null() {
        let mut next = unsafe { (*list).next };

        unsafe {
            dealloc(list as *mut u8, Layout::new::<item>());

            list = next;
        }
    }

    while !list.is_null() {
        let mut next = unsafe { (*list).next };

        unsafe {
            dealloc(list as *mut u8, Layout::new::<item>());
            list = next;
        }
    }

    // memory-leak
}