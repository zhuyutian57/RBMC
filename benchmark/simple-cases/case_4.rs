
struct Node { x : Box<i32> }

fn take_ownership(n : Node) {}

fn main() {
    let mut n = Node { x : Box::new(10) };
    let p = &*n.x as *const i32;
    take_ownership(n);
    let y = unsafe { *p }; // invalid-deref, `n` is moved
}