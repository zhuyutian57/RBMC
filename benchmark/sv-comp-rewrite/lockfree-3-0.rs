use std::{alloc::{alloc, dealloc, Layout}, ptr};

struct cell {
    data : i32,
    next : *mut cell,
}

static mut S : *mut cell = ptr::null_mut();

static mut pc1 : i32 = 1;
static mut pc4 : i32 = 1;

fn push() {
    static mut t1 : *mut cell = ptr::null_mut();
    static mut x1 : *mut cell = ptr::null_mut();
    unsafe { pc1 += 1; }
    match unsafe { pc1 - 1 } {
        1 => {
            unsafe { 
                x1 = alloc(Layout::new::<cell>()) as *mut cell;
                (*x1).data = 0;
                (*x1).next = ptr::null_mut()
            };
        },
        2 => {
            unsafe { (*x1).data = 4; }
            return;
        },
        3 => {
            unsafe { t1 = S };
            return;
        },
        4 => {
            unsafe { (*x1).next = t1 };
            return;
        },
        5 => {
            if unsafe { S == t1 }  {
                unsafe { S = x1; }
            } else {
                unsafe { pc1 = 3; }
            }
            return;
        },
        6 => {
            unsafe { pc1 = 1; }
            return;
        },
        _ => {},
    }
}

static mut garbage : *mut cell = ptr::null_mut();

fn pop() {
    static mut t4 : *mut cell = ptr::null_mut();
    static mut x4 : *mut cell = ptr::null_mut();
    static mut res4 : i32 = 0;

    unsafe { pc4 += 1; }
    match unsafe { pc4 - 1 } {
        1 => {
            unsafe { t4 = S; }
            return;
        },
        2 => {
            unsafe { 
                if t4.is_null() {
                    pc4 = 1;
                }
            }
            return;
        },
        3 => {
            unsafe {
                x4 = (*t4).next;
            }
            return;
        },
        4 => {
            if unsafe { S == t4 }  {
                unsafe { S = x4; }
            } else {
                unsafe { pc4 = 1; }
            }
            return;
        },
        5 => {
            unsafe {
                res4 = (*t4).data;
                (*t4).next = garbage;
                garbage = t4;
                pc4 = 1;
            }
        },
        _ => {},
    }
}

extern crate rbmc;

fn main() {
    while unsafe { 
        !S.is_null() || pc1 != 1 || pc4 != 1
        || rbmc::nondet::<i32>() != 0 // nondeterministic
    } {
        if rbmc::nondet::<i32>() != 0 {
            push();
        } else {
            pop();
        }
    }

    while unsafe { garbage as usize != 0 } {
        let next = unsafe { (*garbage).next };
        unsafe {
            dealloc(garbage as *mut u8, Layout::new::<cell>());
            garbage = next;
        }
    }
}

// safe