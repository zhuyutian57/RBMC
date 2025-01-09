
use std::fmt::Debug;

use crate::expr::context::*;
use crate::expr::expr::*;
use super::value_set::*;

/// Abstract program state for each program point
#[derive(Clone)]
pub struct State {
  pub(super) guard: Expr,
  pub(super) value_set: ValueSet,
}

impl State {
  pub fn new(ctx: ExprCtx) -> Self {
    State {
      guard: ctx.constant_bool(true),
      value_set: ValueSet::default(),
    }
  }

  pub fn guard(&self) -> Expr { self.guard.clone() }

  pub fn add_pointer(&mut self, pt: Expr) {
    assert!(pt.ty().is_any_ptr() && pt.is_symbol());
    let symbol = pt.symbol();
    assert!(symbol.is_level1());
    self.value_set.add(symbol.name());
  }

  pub fn remove_pointer(&mut self, pt: Expr) {
    assert!(pt.ty().is_any_ptr() && pt.is_symbol());
    let symbol = pt.symbol();
    assert!(symbol.is_level1());
    self.value_set.remove(symbol.name());
  }

  pub fn update_value_set(
    &mut self,
    pt: Expr,
    objects: ObjectSet,
    is_union: bool
  ) {
    assert!(pt.ty().is_any_ptr() && pt.is_symbol());
    let symbol = pt.symbol();
    if is_union {
      self.value_set.union(symbol.l1_name(), objects);
    } else {
      self.value_set.insert(symbol.l1_name(), objects);
    }
  }

  pub fn merge(&mut self, other: &State) {
    self.guard = self.guard.ctx.or(self.guard.clone(), other.guard.clone());
    self.guard.simplify();
    self.value_set.merge(&other.value_set, true);
  }

  pub fn get_value_set(&self, expr: &Expr, objects: &mut ObjectSet) {
    
    if expr.is_object() {
      objects.insert(expr.clone());
      return;
    }

    if expr.is_address_of() {
      objects.insert(expr.extract_object());
      return;
    }

    if expr.is_symbol() {
      let pt = expr.symbol().name();
      self.value_set.get(pt, objects);
    }

    //TODO: do more jobs

  }
}

impl Debug for State {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "State -> Guard: {:?}\n{:?}", self.guard, self.value_set)
  }
}