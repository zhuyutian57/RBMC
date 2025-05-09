extern crate rbmc;

struct Node(i32, Option<Box<Node>>);

fn append(list: Node) -> Node {
  Node(0, Some(Box::new(list)))
}

fn main() {
  let mut head = Node(0, None);
  let mut i = 0;
  loop {
    if rbmc::nondet::<bool>() {
      head = append(head);
    }
    i += 1;
    if i == 10 { break; }
  }
}