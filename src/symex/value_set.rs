
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use crate::expr::expr::Expr;
use crate::symbol::nstring::NString;

pub type ObjectSet = HashSet<Expr>;

#[derive(Default, Clone)]
pub(super) struct ValueSet {
  _points_to_map: HashMap<NString, ObjectSet>,
}

impl ValueSet {
  pub fn add(&mut self, pt: NString) {
    self
      ._points_to_map
      .entry(pt)
      .or_default();
  }

  pub fn remove(&mut self, pt: NString) {
    self
      ._points_to_map
      .remove(&pt);
  }
  
  pub fn contains(&self, pt: NString) -> bool {
    self._points_to_map.contains_key(&pt)
  }

  pub fn pointers(&self) -> HashSet<NString> {
    self._points_to_map.keys()
      .map(|x| *x)
      .collect::<HashSet<NString>>()
  }

  pub fn get(&self, pt: NString, objects: &mut ObjectSet) {
    if let Some(s) = self._points_to_map.get(&pt) {
      for object in s { objects.insert(object.clone()); }
    }
  }

  pub fn insert(&mut self, pt: NString, objects: ObjectSet) {
    self._points_to_map.insert(pt, objects);
  }

  pub fn union(&mut self, pt: NString, objects: ObjectSet) {
    let s =
      self
        ._points_to_map
        .entry(pt)
        .or_default();
    for object in objects { s.insert(object); }    
  }

  pub fn remove_stack_places(&mut self, function_name: NString) {
    self
      ._points_to_map
      .retain(|k, _| !k.contains(function_name));
  }
}

impl Debug for ValueSet {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let debug_info = 
      self
        ._points_to_map
        .iter()
        .map(
          |(pt, objects)| { 
            let debug_objects =
            objects
              .iter()
              .map(|x| format!("{x:?}"))
              .collect::<Vec<String>>()
              .join(", ");
            format!("    {pt:?}: {debug_objects}\n")
          }
        )
        .collect::<String>();
    write!(f, "{debug_info}\n")
  }
}