
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;

use crate::expr::context::*;
use crate::expr::expr::*;
use crate::NString;
use super::place_state::*;
use super::renaming::Renaming;
use super::value_set::*;

/// Abstract program state for each program point
#[derive(Clone)]
pub struct State {
  pub(super) guard: Expr,
  pub(super) place_states: PlaceStates,
  pub(super) value_set: ValueSet,
  /// Renaming at some program pointer. Used for
  /// doing phi function while merging states.
  pub(super) renaming: Option<Box<Renaming>>,
}

impl State {
  pub fn new(ctx: ExprCtx) -> Self {
    State {
      guard: ctx.constant_bool(true),
      place_states: PlaceStates::default(),
      value_set: ValueSet::default(),
      renaming: None,
    }
  }

  pub fn guard(&self) -> Expr { self.guard.clone() }

  pub fn update_place_state(&mut self, place: NPlace, state: PlaceState) {
    self.place_states.update(place, state);
  }

  pub fn remove_stack_places(&mut self) {
    self.place_states.remove_stack_places();
    // TODO: handle reborrow
  }

  pub fn add_pointer(&mut self, pt: Expr) {
    assert!(pt.ty().is_any_ptr() && pt.is_symbol());
    let symbol = pt.extract_symbol();
    assert!(symbol.is_level1());
    self.value_set.add(symbol.name());
  }

  pub fn remove_pointer(&mut self, pt: Expr) {
    assert!(pt.ty().is_any_ptr() && pt.is_symbol());
    let symbol = pt.extract_symbol();
    assert!(symbol.is_level1());
    self.value_set.remove(symbol.name());
  }

  pub fn assign(&mut self, pt: Expr, values: ObjectSet) {
    assert!(pt.ty().is_any_ptr() && pt.is_symbol());
    let symbol = pt.extract_symbol();
    self.value_set.insert(symbol.l1_name(), values);
  }

  pub fn merge(&mut self, other: &State) {
    self.guard =
      self.guard.ctx.or(self.guard.clone(), other.guard.clone());
    self.guard.simplify();
    self.place_states.merge(&other.place_states);

    // Merge value set
    let ctx = self.guard.ctx.clone();
    let mut pointers =
      self
        .value_set
        .pointers()
        .union(&other.value_set.pointers())
        .map(|x| *x)
        .collect::<HashSet<_>>();
    for pt in pointers {
      let mut new_objects = HashSet::new();
      self.value_set.get(pt, &mut new_objects);
      other.value_set.get(pt, &mut new_objects);
      // If any of state does not contain the pointer,
      // the pointer is uninitialized.
      if !self.value_set.contains(pt) || !other.value_set.contains(pt) {
        let ty = new_objects.iter().next().unwrap().ty();
        new_objects.insert(ctx.unknown(ty));
      }
      self.value_set.insert(pt, new_objects);
    }
  }

  pub fn get_value_set(&self, expr: Expr, values: &mut ObjectSet) {
    assert!(expr.ty().is_any_ptr());

    if expr.is_null() {
      values.insert(expr.ctx.null_object(expr.ty().pointee_ty()));
      return;
    }

    if expr.is_symbol() {
      let pt = expr.extract_symbol().name();
      self.value_set.get(pt, values);
      return;
    }

    if expr.is_address_of() {
      values.insert(expr.extract_object());
      return;
    }

    if expr.is_cast() {
      let src_expr = expr.extract_src();
      self.get_value_set(src_expr, values);
      return;
    }

    if expr.is_object() {
      let inner_object = expr.extract_inner_expr();
      self.get_value_set(inner_object, values);
      return;
    }

    if expr.is_ite() {
      let true_value = expr.extract_true_value();
      let false_value = expr.extract_false_value();
      self.get_value_set(true_value, values);
      self.get_value_set(false_value, values);
      return;
    }

    panic!("Do not support dereferencing:\n{expr:?}");
  }
}

impl Debug for State {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "State -> Guard: {:?}\n  Place States:\n{:?}\n  Value Set:\n{:?}",
      self.guard,
      self.place_states,
      self.value_set
    )
  }
}