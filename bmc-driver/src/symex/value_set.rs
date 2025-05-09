use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use num_bigint::BigInt;

use crate::expr::expr::Expr;
use crate::symbol::nstring::NString;

pub type Object = (Expr, Option<BigInt>);
pub type ObjectSet = HashSet<Object>;

#[derive(Default, Clone)]
pub(super) struct ValueSet {
    _points_to_map: HashMap<NString, ObjectSet>,
}

impl ValueSet {
    pub fn contains(&self, ident: NString) -> bool {
        self._points_to_map.contains_key(&ident)
    }

    pub fn insert(&mut self, ident: NString, values: ObjectSet, is_union: bool) {
        if is_union {
            self._points_to_map
                .entry(ident)
                .and_modify(|s| {
                    values.iter().for_each(|object| {
                        s.insert(object.clone());
                    })
                })
                .or_insert(values);
        } else {
            self._points_to_map.insert(ident, values);
        }
    }

    pub fn union(&mut self, rhs: &ValueSet) {
        for (&pt, objects) in rhs._points_to_map.iter() {
            self._points_to_map
                .entry(pt)
                .and_modify(|s| {
                    objects.iter().for_each(|o| {
                        s.insert(o.clone());
                    })
                })
                .or_insert(objects.clone());
        }
    }

    pub fn remove(&mut self, ident: NString) {
        self._points_to_map.remove(&ident);
    }

    pub fn get(&self, ident: NString, objects: &mut ObjectSet) {
        if let Some(s) = self._points_to_map.get(&ident) {
            for object in s {
                objects.insert(object.clone());
            }
        }
    }
}

impl Debug for ValueSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let debug_info = self
            ._points_to_map
            .iter()
            .map(|(pt, objects)| {
                let debug_objects =
                    objects.iter().map(|x| format!("{x:?}")).collect::<Vec<String>>().join(", ");
                format!("    {pt:?}: {debug_objects}\n")
            })
            .collect::<String>();
        write!(f, "{debug_info}\n")
    }
}
