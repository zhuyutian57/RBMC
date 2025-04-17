use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Debug;

use super::place_state::*;
use super::renaming::Renaming;
use super::value_set::*;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::guard::Guard;
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::NString;

/// Abstract program state for each program point
#[derive(Clone)]
pub struct State {
    pub ctx: ExprCtx,
    pub(super) guard: Guard,
    pub(super) place_states: PlaceStates,
    pub(super) value_set: ValueSet,
    /// Renaming at some program pointer. Used for doing phi function while merging states.
    pub(super) renaming: Option<RefCell<Renaming>>,
}

impl State {
    pub fn new(ctx: ExprCtx) -> Self {
        State {
            ctx: ctx.clone(),
            guard: Guard::new(ctx.clone()),
            place_states: PlaceStates::default(),
            value_set: ValueSet::default(),
            renaming: None,
        }
    }

    pub fn get_place_state(&self, nplace: NPlace) -> PlaceState {
        self.place_states.place_state(nplace)
    }

    pub fn update_place_state(&mut self, nplace: NPlace, state: PlaceState) {
        self.place_states.update(nplace, state);
    }

    pub fn remove_place(&mut self, nplace: NPlace) {
        self.place_states.remove(nplace);
    }

    pub fn remove_stack_places(&mut self, function_id: NString) {
        self.place_states.remove_stack_places(function_id);
        self.value_set.remove_stack_places(function_id);
        if let Some(renaming) = &self.renaming {
            renaming.borrow_mut().cleanr_locals(function_id);
        }
    }

    pub fn dealloc_objects(&mut self, pt: Expr) {
        assert!(pt.ty().is_any_ptr());
        let mut objects = HashSet::new();
        self.get_value_set(pt.clone(), &mut objects);
        let n = objects.len();
        for (object, _) in objects {
            if object.is_unknown() || object.is_null_object() {
                continue;
            }
            let nplace = NPlace::from(object);
            let mut new_state = PlaceState::Dead;
            if n > 1 {
                new_state.meet(self.get_place_state(nplace));
            }
            self.update_place_state(nplace, new_state);
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
            if values.is_empty() {
                // just clear
                self.value_set.remove(ident);
            } else {
                self.value_set.insert(ident, values);
            }
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
                suffix + if expr.ty().is_array() { format!("[{i}]") } else { format!(".{i}") },
                values,
            );
            return;
        }

        todo!("assign value set for {expr:?}");
    }

    pub fn merge(&mut self, rhs: &State) {
        if self.guard.is_false() {
            self.place_states = rhs.place_states.clone();
            self.value_set = rhs.value_set.clone();
        } else {
            // Merge place states
            self.place_states.merge(&rhs.place_states);
            // Merge value set
            self.value_set.union(&rhs.value_set);
        }
        self.guard |= &rhs.guard;
    }

    pub fn get_value_set(&self, expr: Expr, values: &mut ObjectSet) {
        assert!(expr.ty().is_any_ptr());
        self.get_value_set_rec(expr.clone(), NString::EMPTY, values);
        if values.is_empty() {
            // The pointer points to nothing
            values.insert((expr.ctx.unknown(expr.ty().pointee_ty()), None));
        }
    }

    pub fn get_value_set_rec(&self, expr: Expr, suffix: NString, values: &mut ObjectSet) {
        if expr.is_unknown() {
            values.insert((expr.ctx.unknown(expr.ty().pointee_ty()), None));
            return;
        }

        if expr.is_null() {
            values.insert((expr.ctx.null_object(expr.ty().pointee_ty()), None));
            return;
        }

        if expr.is_symbol() {
            let pt = expr.extract_symbol().name();
            let ident = pt + suffix;
            self.value_set.get(ident, values);
            return;
        }

        if expr.is_address_of() {
            let object = expr.extract_object();
            let inner_expr = object.extract_inner_expr();
            if inner_expr.is_symbol() || inner_expr.is_slice() {
                values.insert((object, None));
            } else if inner_expr.is_index() {
                let root_object = inner_expr.extract_object();
                let index = inner_expr.extract_index().extract_constant();
                let offset = index.to_integer();
                values.insert((root_object, Some(offset)));
            } else {
                todo!("get value set from addressof({object:?})");
            }
            return;
        }

        if expr.is_offset() {
            let pt = expr.extract_root_pointer();
            let off = expr.extract_offset();
            // TODO: support dynamic offset
            assert!(off.is_constant());
            let offset = off.extract_constant().to_integer();
            let mut objects = HashSet::new();
            self.get_value_set_rec(pt, suffix, &mut objects);
            // Compute new offset
            for (object, o) in objects {
                let new_offset = match o {
                    Some(x) => offset.clone() + x,
                    None => offset.clone(),
                };
                values.insert((object, Some(new_offset)));
            }
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
            let i = bigint_to_usize(&expr.extract_index().extract_constant().to_integer());
            let new_suffix =
                suffix + if expr.ty().is_array() { format!("[{i}]") } else { format!(".{i}") };
            if inner_expr.is_symbol() {
                self.get_value_set_rec(inner_expr.clone(), new_suffix, values);
            } else if inner_expr.is_aggregate() {
                let fields = inner_expr.extract_fields();
                assert!(i < fields.len());
                self.get_value_set_rec(fields[i].clone(), suffix, values);
            } else if inner_expr.is_store() {
                let inner_object = inner_expr.extract_object();
                let update_index = inner_expr.extract_index();
                let j = bigint_to_usize(&update_index.extract_constant().to_integer());
                let update_value = inner_expr.extract_update_value();
                if i == j {
                    self.get_value_set_rec(update_value, suffix, values);
                } else {
                    self.get_value_set_rec(inner_object, new_suffix, values);
                }
            } else if inner_expr.is_ite() || inner_expr.is_index() {
                self.get_value_set_rec(inner_expr, new_suffix, values);
            } else if inner_expr.is_unknown() {
                values.insert((expr.ctx.unknown(expr.ty().pointee_ty()), None));
            } else {
                panic!("Wrong object? {inner_expr:?}");
            }
            return;
        }

        if expr.is_vec() || expr.is_inner_pointer() {
            let inner_pt = expr.extract_inner_pointer();
            self.get_value_set_rec(inner_pt, suffix, values);
            return;
        }

        if expr.is_move() {
            self.get_value_set_rec(expr.extract_object(), suffix, values);
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
            self.guard, self.place_states, self.value_set
        )
    }
}
