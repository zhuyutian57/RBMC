use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Debug;

use super::place_state::*;
use super::renaming::Renaming;
use super::value_set::*;
use crate::expr::constant::Constant;
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
        assert!(pt.ty().is_primitive_ptr());
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
        assert!(pt.ty().is_primitive_ptr());
        let ident = NString::from(format!("{pt:?}"));
        self.value_set.remove(ident);
    }

    pub fn assign(&mut self, expr: Expr, values: ObjectSet) {
        assert!(expr.ty().is_primitive_ptr());
        self.assign_rec(expr, NString::EMPTY, values);
    }

    fn assign_rec(&mut self, expr: Expr, suffix: NString, values: ObjectSet) {
        if expr.is_symbol() {
            let symbol = expr.extract_symbol();
            let ident = symbol.name() + suffix;
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

        assert!(expr.ty().is_primitive_ptr());

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
        assert!(expr.ty().is_primitive_ptr());
        self.get_value_set_rec(expr.clone(), NString::EMPTY, values);
        if values.is_empty() {
            // The pointer points to nothing
            values.insert((expr.ctx.unknown(expr.ty().pointee_ty()), None));
        }
    }

    pub fn get_value_set_rec(&self, expr: Expr, suffix: NString, values: &mut ObjectSet) {
        if expr.is_unknown() || expr.is_invalid_object() {
            values.insert((expr.ctx.unknown(expr.ty().pointee_ty()), None));
            return;
        }

        if expr.is_null() {
            values.insert((expr.ctx.null_object(), None));
            return;
        }

        if expr.is_symbol() {
            let pt = expr.extract_symbol().name();
            let ident = pt + suffix;
            self.value_set.get(ident, values);
            return;
        }

        if expr.is_address_of() {
            let expr = expr.extract_object();
            let mut object_set = ObjectSet::new();
            self.get_object_rec(expr, &mut object_set);
            for (object, _) in object_set {
                let inner_expr = object.extract_inner_expr();
                if inner_expr.is_index() {
                    let o = inner_expr.extract_object();
                    let i = inner_expr.extract_index().extract_constant();
                    let offset = bigint_to_usize(&i.to_integer());
                    values.insert((o, Some(offset.into())));
                } else {
                    values.insert((object, None));
                }
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
            let index = expr.extract_index().extract_constant();
            let inner_expr = expr.extract_object().extract_inner_expr();
            let i = bigint_to_usize(&index.to_integer());
            let mut object_set = ObjectSet::new();
            self.get_object_rec(inner_expr.clone(), &mut object_set);
            let new_suffix = NString::from(if inner_expr.ty().is_array() {
                format!("[{i}]")
            } else if inner_expr.ty().is_tuple() {
                format!(".{i}")
            } else {
                assert!(inner_expr.ty().is_struct());
                let field = inner_expr.ty().struct_def().1[i].0;
                format!(".{field:?}")
            }) + suffix;
            for (object, _) in object_set {
                let _inner_expr = object.extract_inner_expr();
                if _inner_expr.is_aggregate() {
                    let fields = _inner_expr.extract_fields();
                    assert!(i < fields.len());
                    self.get_value_set_rec(fields[i].clone(), suffix, values);
                } else if _inner_expr.is_constant() {
                    let (fields, _) = _inner_expr.extract_constant().to_adt();
                    let j = if _inner_expr.ty().is_enum() { i + 1 } else { i };
                    let field = self.ctx.constant(fields[j].clone(), expr.ty());
                    self.get_value_set_rec(field, suffix, values);
                } else if _inner_expr.is_unknown() || _inner_expr.is_null_object() {
                    values.insert((_inner_expr.ctx.unknown(_inner_expr.ty().pointee_ty()), None));
                } else if _inner_expr.is_store() {
                    let _index = _inner_expr.extract_index().extract_constant();
                    let j = bigint_to_usize(&_index.to_integer());
                    if i == j {
                        let _object = _inner_expr.extract_update_value();
                        self.get_value_set_rec(_object, suffix, values);
                    } else {
                        let _object = _inner_expr.extract_object();
                        self.get_value_set_rec(_object, new_suffix, values);
                    }
                } else {
                    self.get_value_set_rec(_inner_expr, new_suffix, values);
                }
            }
            return;
        }

        if expr.is_pointer() || expr.is_pointer_base() {
            self.get_value_set_rec(expr.extract_inner_pointer(), suffix, values);
            return;
        }

        if expr.is_offset() {
            let mut objects = HashSet::new();
            self.get_value_set_rec(expr.extract_lhs(), suffix, &mut objects);
            let rhs = expr.extract_rhs();
            // TODO: support dynamic offset
            assert!(rhs.is_constant());
            // Compute new offset.
            let offset = rhs.extract_constant().to_integer();
            for (object, o) in objects {
                let new_offset = match o {
                    Some(x) => offset.clone() + x,
                    None => offset.clone(),
                };
                values.insert((object, Some(new_offset)));
            }
            return;
        }

        if expr.is_move() {
            self.get_value_set_rec(expr.extract_object(), suffix, values);
            return;
        }

        panic!("Do not support dereferencing:\n{expr:?}");
    }

    /// Get objects from current expr. Similar to get_reference_rec in ESBMC.
    ///
    /// TODO: support more expr
    fn get_object_rec(&self, expr: Expr, object_set: &mut ObjectSet) {
        if expr.is_symbol()
            || expr.is_constant()
            || expr.is_aggregate()
            || expr.is_slice()
            || expr.is_store()
            || expr.is_as_variant()
        {
            object_set.insert((self.ctx.object(expr), None));
            return;
        }

        if expr.is_ite() {
            let true_value = expr.extract_true_value();
            let false_value = expr.extract_false_value();
            self.get_object_rec(true_value, object_set);
            self.get_object_rec(false_value, object_set);
            return;
        }

        if expr.is_cast() {
            self.get_object_rec(expr.extract_src(), object_set);
            return;
        }

        if expr.is_object() {
            self.get_object_rec(expr.extract_inner_expr(), object_set);
            return;
        }

        if expr.is_index() {
            // TODO; support dynamic offset?
            let index = expr.extract_index();
            let i = bigint_to_usize(&index.extract_constant().to_integer());
            let mut root_objects = ObjectSet::new();
            self.get_object_rec(expr.extract_object(), &mut root_objects);
            for (o, _) in root_objects {
                let root_object = o.extract_inner_expr();
                let mut object = if root_object.is_aggregate() {
                    root_object.extract_fields()[i].clone()
                } else if root_object.is_constant() {
                    let constant = match root_object.extract_constant() {
                        Constant::Array(c, _) => *c,
                        Constant::Adt(fields, ty) => {
                            if ty.is_enum() {
                                fields[i + 1].clone()
                            } else {
                                fields[i].clone()
                            }
                        }
                        _ => panic!("Impossible"),
                    };
                    self.ctx.constant(constant, expr.ty())
                } else if root_object.is_store() {
                    let stored_object = root_object.extract_object();
                    let update_index = root_object.extract_index();
                    let update_value = root_object.extract_update_value();
                    let j = bigint_to_usize(&update_index.extract_constant().to_integer());
                    if i == j {
                        update_value
                    } else {
                        self.ctx.index(stored_object, index.clone(), expr.ty())
                    }
                } else {
                    self.ctx.index(root_object, index.clone(), expr.ty())
                };
                if !object.is_object() {
                    object = self.ctx.object(object);
                }
                object_set.insert((object, None));
            }
            return;
        }

        if expr.is_invalid_object() {
            object_set.insert((self.ctx.object(expr), None));
            return;
        }

        panic!("Do not support get object from: {expr:?}")
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
