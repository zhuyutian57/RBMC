use std::ptr;

struct Node {
    x : i32,
    nxt : *mut Node,
}

fn prepend(n : *mut Node) -> Node {
    Node { x : 0, nxt : n }
}

// #[kani::proof]
fn main() {
    let mut hn = 
        &mut Node {
            x : 1,
            nxt : unsafe { ptr::null_mut() }
        } as *mut Node;
    hn = &mut prepend(hn) as *mut Node; // the ret is freed here
    unsafe{ *hn = Node { x : 1, nxt : ptr::null_mut() } }; // invalid-deref
}