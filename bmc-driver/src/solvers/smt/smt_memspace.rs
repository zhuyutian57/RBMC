use std::collections::HashMap;

use crate::expr::expr::*;

pub type ObjectSpace<Ast> = (Ast, Ast);

/// The space of an object is identified by `[star, end)`,
pub struct PointerLogic<Ast: Clone> {
    objects: HashMap<Expr, usize>,
    object_spaces: HashMap<usize, ObjectSpace<Ast>>,
}

impl<Ast: Clone> PointerLogic<Ast> {
    pub fn new() -> Self {
        PointerLogic { objects: HashMap::new(), object_spaces: HashMap::new() }
    }

    pub fn add_object(&mut self, object: Expr) -> usize {
        let n = self.objects.len();
        *self.objects.entry(object).or_insert(n)
    }

    pub fn contains(&self, object: &Expr) -> bool {
        self.objects.contains_key(object)
    }

    pub fn clear(&mut self) {
        self.objects.clear();
        self.object_spaces.clear();
    }

    pub fn set_object_space(&mut self, object: Expr, space: ObjectSpace<Ast>) {
        assert!(self.contains(&object));
        let i = self.get_object_space_ident(&object);
        self.object_spaces.insert(i, space);
    }

    pub fn object_spaces(&self) -> &HashMap<usize, ObjectSpace<Ast>> {
        &self.object_spaces
    }

    pub fn get_object_space_ident(&self, object: &Expr) -> usize {
        assert!(self.contains(object));
        *self.objects.get(object).unwrap()
    }
}

pub trait MemSpace<Sort, Ast> {
    fn set_pointer_logic(&mut self);

    fn pointer_sort(&self) -> Sort;

    fn create_object_space(&mut self, object: &Expr) -> Ast;
    fn init_pointer_space(&mut self, object: &Expr);

    fn mk_pointer(&self, base: &Ast, offset: &Ast, meta: Option<&Ast>) -> Ast;
    fn mk_pointer_base(&self, pt: &Ast) -> Ast;
    fn mk_pointer_offset(&self, pt: &Ast) -> Ast;
    fn mk_pointer_meta(&self, pt: &Ast) -> Ast;
}
