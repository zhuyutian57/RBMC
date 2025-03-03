
use std::collections::HashMap;
use std::fmt::Debug;

use crate::symbol::nstring::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaceState {
  Uninitialized,
  Moved,
  Initialized,
}

impl PlaceState {
  /// Meet operator
  pub fn merge(s1: PlaceState, s2: PlaceState) -> Self {
    if s1 < s2 { s1 } else { s2 }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKind {
  Stack,
  Heap,
}

impl From<NString> for PlaceKind {
  fn from(value: NString) -> Self {
    if value.contains(NString::from("heap")) {
      PlaceKind::Heap
    } else {
      PlaceKind::Stack
    }
  }
}

/// Add a kind flag
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NPlace(PlaceKind, NString);

impl NPlace {
  pub fn new(kind: PlaceKind, ident: NString) -> Self {
    NPlace(kind, ident)
  }

  pub fn kind(&self) -> PlaceKind { self.0 }

  pub fn place(&self) -> NString { self.1 }
}

impl Debug for NPlace {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "({:?}, {:?})", self.0, self.1)
  }
}

pub type PlaceStateMap = HashMap<NPlace, PlaceState>;

#[derive(Clone, Default)]
pub struct PlaceStates {
  _place_states_map: PlaceStateMap,
}

impl PlaceStates {
  pub fn place_states(&self) -> &PlaceStateMap {
    &self._place_states_map
  }

  pub fn local_place_states(&self) -> PlaceStateMap {
    self
      ._place_states_map
      .iter()
      .filter(|&(p, _)| matches!(p.0, PlaceKind::Stack))
      .map(|(p, s)| (p.clone(), s.clone()))
      .collect()
  }

  pub fn heap_place_states(&self) -> PlaceStateMap {
    self
      ._place_states_map
      .iter()
      .filter(|&(p, _)| matches!(p.0, PlaceKind::Stack))
      .map(|(p, s)| (p.clone(), s.clone()))
      .collect()
  }

  pub fn update(&mut self, place: NPlace, state: PlaceState) {
    self
      ._place_states_map
      .entry(place)
      .and_modify(|s| *s = state)
      .or_insert(state);
  }

  pub fn merge(&mut self, rhs: &PlaceStates) {
    for (&place, &state) in rhs.place_states().iter() {
      self
        ._place_states_map
        .entry(place)
        .and_modify(
          |s|
          *s = PlaceState::merge(*s, state))
        .or_insert(state);
    }
  }

  pub fn remove_stack_places(&mut self) {
    self
      ._place_states_map
      .retain(|k, _| matches!(k.kind(), PlaceKind::Heap));
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