
use std::cell::RefCell;
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
  pub(super) renaming: Option<RefCell<Renaming>>,
}

impl State {
  pub fn new(ctx: ExprCtx) -> Self {
    State {
      guard: ctx._true(),
      place_states: PlaceStates::default(),
      value_set: ValueSet::default(),
      renaming: None,
    }
  }

  pub fn guard(&self) -> Expr { self.guard.clone() }

  pub fn update_place_state(&mut self, place: NPlace, state: PlaceState) {
    self.place_states.update(place, state);
  }

  pub fn remove_place(&mut self, place: NPlace) {
    self.place_states.remove(place);
  }

  pub fn remove_stack_places(&mut self, function_name: NString) {
    self.place_states.remove_stack_places(function_name);
    self.value_set.remove_stack_places(function_name);
  }

  pub fn dealloc_objects(&mut self, pt: Expr) {
    assert!(pt.ty().is_any_ptr());
    let mut objects = HashSet::new();
    self.get_value_set(pt.clone(), &mut objects);
    for object in objects {
      if object.is_unknown() { continue; }
      let place = NPlace::from(object);
      self.update_place_state(place, PlaceState::Dealloced);
    }
  }

  pub fn remove_pointer(&mut self, pt: Expr) {
    assert!(pt.ty().is_any_ptr());
    let ident = NString::from(format!("{pt:?}"));
    self.value_set.remove(ident);
  }

  pub fn assign(&mut self, expr: Expr, values: ObjectSet) {
    assert!(expr.ty().is_any_ptr());
    self.assign_rec(expr, NString::EMPTY, values);
  }

  fn assign_rec(&mut self, expr: Expr, suffix: NString, values: ObjectSet) {
    if expr.is_symbol() {
      let symbol = expr.extract_symbol();
      let ident = symbol.l1_name() + suffix;
      self.value_set.insert(ident, values.clone());
      return;
    }

    if expr.is_object() {
      let inner_expr = expr.extract_inner_expr();
      self.assign_rec(inner_expr, suffix, values);
      return;
    }

    assert!(expr.ty().is_any_ptr());

    if expr.is_index() {
      let object = expr.extract_object();
      let index_str = format!("{:?}", expr.extract_index());
      let i = index_str.parse::<u128>().expect("Not integer index");
      self.assign_rec(
        object,
        suffix + 
          if expr.ty().is_array() {
            format!("[{i}]")
          } else {
            format!(".{i}")
          },
        values.clone());
      return;
    }

    todo!("assign value set for {expr:?}");
  }

  pub fn merge(&mut self, other: &State) {
    let ctx = self.guard.ctx.clone();
    if self.guard.is_false() {
      self.place_states = other.place_states.clone();
      self.value_set = other.value_set.clone();
    } else {
      // Merge place states
      self.place_states.merge(&other.place_states);
      // Merge value set
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
    self.guard = ctx.or(self.guard.clone(), other.guard.clone());
    self.guard.simplify();
  }

  pub fn get_value_set(&self, expr: Expr, values: &mut ObjectSet) {
    assert!(expr.ty().is_any_ptr());
    self.get_value_set_rec(expr.clone(), NString::EMPTY, values);
    if values.is_empty() {
      // The pointer points to nothing
      values.insert(expr.ctx.unknown(expr.ty().pointee_ty()));
    }
  }

  pub fn get_value_set_rec(
    &self,
    expr: Expr,
    suffix: NString,
    values: &mut ObjectSet
  ) {
    if expr.is_null() {
      values.insert(expr.ctx.null_object(expr.ty().pointee_ty()));
      return;
    }

    if expr.is_symbol() {
      let pt = expr.extract_symbol().name();
      let ident = pt + suffix;
      self.value_set.get(ident, values);
      return;
    }

    if expr.is_address_of() {
      values.insert(expr.extract_object());
      return;
    }

    if expr.is_ite() {
      let true_value = expr.extract_true_value();
      let false_value = expr.extract_false_value();
      self.get_value_set_rec(true_value, suffix, values);
      self.get_value_set_rec(false_value, suffix, values);
      return;
    }

    if expr.is_cast() {
      let src_expr = expr.extract_src();
      self.get_value_set_rec(src_expr, suffix, values);
      return;
    }

    if expr.is_object() {
      let inner_object = expr.extract_inner_expr();
      self.get_value_set_rec(inner_object, suffix, values);
      return;
    }

    if expr.is_index() {
      let inner_expr = expr.extract_object().extract_inner_expr();
      let index_str = format!("{:?}", expr.extract_index());
      let i = index_str.parse::<usize>().expect("Not integer index");
      if inner_expr.is_symbol() {
        let new_suffix = 
          suffix + 
            if expr.ty().is_array() {
              format!("[{i}]")
            } else {
              format!(".{i}")
            };
        self.get_value_set_rec(inner_expr.clone(), new_suffix, values);
      } else if inner_expr.is_aggregate() {
        let fields = inner_expr.extract_fields();
        assert!(i < fields.len());
        self.get_value_set_rec(fields[i].clone(), suffix, values);
      } else {
        panic!("Wrong object?");
      }
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