
use std::cmp::min;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::expr::expr::Expr;
use crate::symbol::nstring::*;

/// `Place State` is the abstraction of the ownership of
/// a piece of memory(an object).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaceState {
  /// We don't know whether the place is alloced, owned
  /// or dropped(dealloced). Let SMT solve the puzzle.
  Unknown,
  /// The place is alloced(valid) in memory, but not owned
  /// by any variable.
  Alloced,
  /// The place is owned by some variables or in stack.
  Own,
}

impl PlaceState {
  pub fn is_unknown(&self) -> bool {
    matches!(self, PlaceState::Unknown)
  }

  pub fn is_alloced(&self) -> bool {
    matches!(self, PlaceState::Alloced)
  }

  pub fn is_own(&self) -> bool {
    matches!(self, PlaceState::Own)
  }
  
  /// The meet operation. A place is owned by some variables(or frame)
  /// only if two program states own the place. Otherwise, we mark its
  /// state `alloced`.
  /// 
  /// TODO: design carefully.
  pub fn merge(s1: PlaceState, s2: PlaceState) -> Self {
    min(s1, s2)
  }
}

/// Add a kind flag
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NPlace(pub NString);

impl Debug for NPlace {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.0)
  }
}

impl From<Expr> for NPlace {
  fn from(value: Expr) -> Self {
    NPlace(NString::from(format!("{value:?}")))
  }
}

pub type PlaceStateMap = HashMap<NPlace, PlaceState>;

#[derive(Clone, Default)]
pub struct PlaceStates {
  _place_states_map: PlaceStateMap,
}

impl PlaceStates {
  pub fn place_state(&self, place: &Expr) -> PlaceState {
    let nplace = NPlace(NString::from(format!("{place:?}")));
    self
      ._place_states_map
      .get(&nplace)
      .map_or(PlaceState::Unknown, |x| *x)
  }

  pub fn update(&mut self, nplace: NPlace, state: PlaceState) {
    self
      ._place_states_map
      .entry(nplace)
      .and_modify(|s| *s = state)
      .or_insert(state);
  }

  pub fn remove(&mut self, nplace: NPlace) {
    self._place_states_map.remove(&nplace);
  }

  pub fn merge(&mut self, rhs: &PlaceStates) {
    for (&place, &state) in rhs._place_states_map.iter() {
      self
        ._place_states_map
        .entry(place)
        .and_modify(
          |s|
          *s = PlaceState::merge(*s, state))
        .or_insert(PlaceState::Unknown);
    }
  }

  pub fn remove_stack_places(&mut self, function_id: NString) {
    self
      ._place_states_map
      .retain(|p,_| !p.0.contains(function_id));
  }
}

impl Debug for PlaceStates {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let state = 
      self
        ._place_states_map
        .iter()
        .map(|(p, s)| format!("    {p:?}: {s:?}\n"))
        .collect::<String>();
    write!(f, "{state}")
  }
}