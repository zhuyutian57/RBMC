
struct Node(i32, i32);

fn main() {
  let x = &mut Node(1, 2);
  // reborrow x to two disjoint subfields
  let y = &mut x.0;
  let z = &mut x.1;
  // y and z are now live, but x isn't
  *y = 3;
  *z = 4;
  // y and z go out of scope, so x is live again
  *x = Node(5, 7);
}