
struct Node { x : Box<i32> }

fn take_ownership(n : Node) {}

fn main() {
    let mut n = Node { x : Box::new(10) };
    let p = &mut n as *mut Node;
    take_ownership(n);
    unsafe { (*p).x = Box::new(100); } // invalid-free, double-free
}