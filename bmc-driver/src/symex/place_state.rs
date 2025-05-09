use std::cmp::min;
use std::collections::HashMap;
use std::fmt::Debug;

use crate::symbol::nstring::*;

/// `Place State` is the abstraction of the ownership of
/// a piece of memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlaceState {
    /// We don't know whether the place is alloced, owned
    /// or dropped(dealloced). Let SMT solve the puzzle.
    Unknown,
    /// The place(object) is dead(dealloced, dropped).
    Dead,
    /// The place(object) is valid in memory,
    /// but not owned by any variable.
    Alive,
    /// The place is owned by some variables or in stack.
    Own,
}

impl PlaceState {
    pub fn is_unknown(&self) -> bool {
        matches!(self, PlaceState::Unknown)
    }

    pub fn is_dead(&self) -> bool {
        matches!(self, PlaceState::Dead)
    }

    pub fn is_alive(&self) -> bool {
        matches!(self, PlaceState::Alive)
    }

    pub fn is_own(&self) -> bool {
        matches!(self, PlaceState::Own)
    }

    pub fn is_valid(&self) -> bool {
        self.is_alive() || self.is_own()
    }

    /// The meet operation. A place is owned by some variables(or frame)
    /// only if two program states own the place. Otherwise, we mark its
    /// state `alloced`.
    ///
    /// TODO: design carefully.
    pub fn meet(&mut self, rhs: PlaceState) {
        *self = if self.is_valid() && rhs.is_dead() || self.is_dead() && rhs.is_valid() {
            PlaceState::Unknown
        } else {
            min(*self, rhs)
        };
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

pub type PlaceStateMap = HashMap<NPlace, PlaceState>;

#[derive(Clone, Default)]
pub struct PlaceStates {
    _place_states_map: PlaceStateMap,
}

impl PlaceStates {
    pub fn place_state(&self, nplace: NPlace) -> PlaceState {
        self._place_states_map.get(&nplace).map_or(PlaceState::Dead, |x| *x)
    }

    pub fn update(&mut self, nplace: NPlace, state: PlaceState) {
        if state.is_dead() {
            self._place_states_map.remove(&nplace);
        } else {
            self._place_states_map.entry(nplace).and_modify(|s| *s = state).or_insert(state);
        }
    }

    pub fn remove(&mut self, nplace: NPlace) {
        self._place_states_map.remove(&nplace);
    }

    pub fn merge(&mut self, rhs: &PlaceStates) {
        for (&place, &state) in rhs._place_states_map.iter() {
            self._place_states_map
                .entry(place)
                .and_modify(|s| s.meet(state))
                // For a place that is not recorded in current set, the state should be
                // unknown and solved by SMT solver.
                .or_insert(state);
        }
    }
}

impl Debug for PlaceStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self
            ._place_states_map
            .iter()
            .map(|(p, s)| format!("    {p:?}: {s:?}\n"))
            .collect::<String>();
        write!(f, "{state}")
    }
}
