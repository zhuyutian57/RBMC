
use std::collections::HashMap;

use crate::expr::expr::*;
use crate::expr::ty::*;

pub type ObjectSpace<Ast> = (Ast, (Ast, Ast));

/// The space of an object is identified by `(ident, (base, len))`,
/// where `ident` is a natural variable for identification and
/// `(base, len)` is used to model the size of the space.
/// Moreover, it's not neccessary th make `ident = base`.
pub struct PointerLogic<Ast: Clone> {
  _object_spaces: HashMap<Expr, ObjectSpace<Ast>>,
}

impl<Ast: Clone> PointerLogic<Ast> {
  pub fn new() -> Self {
    PointerLogic { _object_spaces: HashMap::new() }
  }

  pub fn contains(&self, object: &Expr) -> bool {
    self._object_spaces.contains_key(object)
  }

  pub fn clear(&mut self) { self._object_spaces.clear(); }

  pub fn set_object_space(
    &mut self,
    object: Expr,
    space: ObjectSpace<Ast>) {
    assert!(!self.contains(&object));
    self._object_spaces.insert(object, space);
  }

  pub fn object_spaces(&self) -> &HashMap<Expr, ObjectSpace<Ast>> {
    &self._object_spaces
  }

  pub fn get_object_space_ident(&self, object: &Expr) -> Ast {
    self
      ._object_spaces
      .get(object)
      .expect(format!("Object space dose not have {object:?}").as_str())
      .0
      .clone()
  }

  pub fn get_object_space_base(&self, object: &Expr) -> Ast {
    self
      ._object_spaces
      .get(object)
      .expect(format!("Object space dose not have {object:?}").as_str())
      .1
      .0
      .clone()
  }

  pub fn get_object_space_len(&self, object: &Expr) -> Ast {
    self
      ._object_spaces
      .get(object)
      .expect(format!("Object space dose not have {object:?}").as_str())
      .1
      .1
      .clone()
  }
}

pub trait MemSpace<Sort, Ast> {
  fn set_pointer_logic(&mut self);
  
  fn pointer_sort(&self) -> Sort;
  fn box_sort(&self) -> Sort;
  fn slice_ptr_sort(&self) -> Sort;
  
  fn create_object_space(&mut self, object: &Expr) -> Ast;
  fn init_pointer_space(&mut self, object: &Expr);

  fn mk_pointer(&self, ident: &Ast, offset: &Ast) -> Ast;
  fn mk_pointer_ident(&self, pt: &Ast) -> Ast;
  fn mk_pointer_offset(&self, pt: &Ast) -> Ast;
  fn mk_box(&self, inner_pt: &Ast) -> Ast;
  fn mk_box_ptr(&self, _box: &Ast) -> Ast;
  fn mk_slice(&self, pt: &Ast, meta: &Ast) -> Ast;
  fn mk_slice_ptr(&self, slice: &Ast) -> Ast;
  fn mk_slice_meta(&self, slice: &Ast) -> Ast; 
}