
struct Node { x : Box<i32> }

fn take_ownership(n : Node) {}

// #[kani::proof]
fn main() {
    let mut n = Node { x : Box::new(10) };
    let p = &mut n as *mut Node;
    take_ownership(n);
    let y = unsafe { *(*p).x }; // invalid-deref, use-after-free
}