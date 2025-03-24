use std::{{borrow::Borrow}, cell::RefCell, rc::Rc};

enum List {
    Cons(i32, RefCell<Rc<List>>),
    Nil,
}

// #[kani::proof]
fn main() {
    let mut n1 = Rc::new(
        List::Cons(1, RefCell::new(Rc::new(List::Nil)))
    );
    let mut n2 = Rc::new(
        List::Cons(2, RefCell::new(n1.clone()))
    );
    match n1.borrow() {
        List::Cons(x, nxt) => {
            *nxt.borrow_mut() = n2.clone(); 
        },
        _ => {}
    }
    // cyclic linked list
    // `RefCell` has ability of interior multibility
    // `Rc` will drop the memory it holds when the counting is 0
    // the countings of `n1` and `n2` are `2`
    // because of the cyclic links between them
    // memory-leak
}