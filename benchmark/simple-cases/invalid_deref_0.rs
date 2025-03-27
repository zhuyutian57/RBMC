
struct Node { x : Box<i32> }

// `n` is dropped in `take_ownership`
// since the ownership of `n.x`` is captured
// `n.x` is dropped
fn take_ownership(n : Node) {}

fn main() {
    let mut n = Node { x : Box::new(10) };
    let p = &mut n as *mut Node;
    take_ownership(n);
    unsafe { *(*p).x = 13; } // invalid-deref
}