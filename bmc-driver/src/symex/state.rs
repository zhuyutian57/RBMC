use std::collections::HashSet;
use std::fmt::Debug;

use super::place_state::*;
use super::renaming::Renaming;
use super::value_set::*;
use crate::expr::constant::Constant;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::guard::Guard;
use crate::expr::ty::Type;
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::NString;

/// Abstract program state for each program point
#[derive(Clone)]
pub struct State {
    pub(super) ctx: ExprCtx,
    pub(super) guard: Guard,
    pub(super) place_states: PlaceStates,
    pub(super) value_set: ValueSet,
    /// Renaming at some program pointer. Used for doing phi function while merging states.
    pub(super) renaming: Option<Renaming>,
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

    pub fn dealloc_objects(&mut self, pt: Expr) {
        assert!(pt.ty().is_primitive_ptr());
        let mut objects = HashSet::new();
        self.get_value_set(pt.clone(), &mut objects);
        let n = objects.len();
        for (object, _) in objects {
            if object.is_unknown() || object.is_null_object() {
                continue;
            }
            let inner_expr = object.extract_inner_expr();
            if !inner_expr.is_symbol() {
                continue;
            }
            let symbol = inner_expr.extract_symbol();
            if symbol.is_stack_symbol() {
                continue;
            }
            let nplace = NPlace(symbol.l1_name());
            let mut new_state = PlaceState::Dead;
            if n > 1 {
                new_state.meet(self.get_place_state(nplace));
            }
            self.update_place_state(nplace, new_state);
        }
    }

    pub fn remove_pointer(&mut self, pt: Expr) {
        assert!(pt.ty().is_primitive_ptr());
        self.remove_pointer_by(NString::from(format!("{pt:?}")));
    }

    pub fn remove_pointer_by(&mut self, pt: NString) {
        self.value_set.remove(pt);
    }

    pub fn assign(&mut self, ident: NString, values: ObjectSet, is_union: bool) {
        if values.is_empty() && !is_union {
            self.value_set.remove(ident);
        } else {
            self.value_set.insert(ident, values, is_union);
        }
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

        if expr.is_constant() {
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
            self.get_object_rec(expr, values);
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
            let new_suffix = if inner_expr.ty().is_array() {
                NString::from(format!("[{i}]"))
            } else if inner_expr.ty().is_tuple() {
                NString::from(format!(".{i}"))
            } else if inner_expr.ty().is_struct() {
                NString::from(format!(".{:?}", inner_expr.ty().struct_def().1[i].0))
            } else {
                assert!(inner_expr.ty().is_enum());
                // Let AsVariant to solve the suffix
                NString::EMPTY
            } + suffix;
            if inner_expr.is_aggregate() {
                let fields = inner_expr.extract_fields();
                assert!(i < fields.len());
                self.get_value_set_rec(fields[i].clone(), suffix, values);
            } else if inner_expr.is_constant() {
                let (fields, _) = inner_expr.extract_constant().to_adt();
                let j = if inner_expr.ty().is_enum() { i + 1 } else { i };
                let field = self.ctx.constant(fields[j].clone(), expr.ty());
                self.get_value_set_rec(field, suffix, values);
            } else if inner_expr.is_unknown() || inner_expr.is_null_object() {
                values.insert((inner_expr.ctx.unknown(inner_expr.ty().pointee_ty()), None));
            } else if inner_expr.is_store() {
                let _index = inner_expr.extract_index().extract_constant();
                let j = bigint_to_usize(&_index.to_integer());
                if i == j {
                    let _object = inner_expr.extract_update_value();
                    self.get_value_set_rec(_object, suffix, values);
                } else {
                    let _object = inner_expr.extract_object();
                    self.get_value_set_rec(_object, new_suffix, values);
                }
            } else {
                self.get_value_set_rec(inner_expr, new_suffix, values);
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

        if expr.is_as_variant() {
            let _enum = expr.extract_enum();
            let i = expr.extract_variant_idx();
            let new_suffix = NString::from(format!(".data({i})")) + suffix;
            self.get_value_set_rec(_enum, new_suffix, values);
            return;
        }

        if expr.is_move() {
            self.get_value_set_rec(expr.extract_object(), suffix, values);
            return;
        }

        panic!("Do not support dereferencing:\n{expr:?} with {suffix:?}");
    }

    /// Get objects from current expr. Similar to get_reference_rec in ESBMC.
    fn get_object_rec(&self, expr: Expr, object_set: &mut ObjectSet) {
        if expr.is_symbol()
            || expr.is_constant()
            || expr.is_aggregate()
            || expr.is_slice()
            || expr.is_store()
            || expr.is_variant()
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
            let mut inner_objects = ObjectSet::new();
            self.get_object_rec(expr.extract_object(), &mut inner_objects);
            let inner_object_ty = expr.extract_object().ty();
            for (inner_object, offset) in inner_objects {
                let new_object = if let Some(off) = offset {
                    let idx = self.ctx.constant_integer(off, Type::usize_type());
                    self.ctx.index(inner_object, idx, inner_object_ty)
                } else {
                    inner_object
                };
                let final_object =
                    if !new_object.is_object() { self.ctx.object(new_object) } else { new_object };
                object_set.insert((final_object, Some(i.into())));
            }
            return;
        }

        if expr.is_as_variant() {
            let variant_idx = expr.extract_variant_idx();
            let mut inner_objects = ObjectSet::new();
            self.get_object_rec(expr.extract_enum(), &mut inner_objects);
            let inner_object_ty = expr.ty();
            for (inner_object, offset) in inner_objects {
                let new_object = if let Some(off) = offset {
                    let idx = self.ctx.constant_integer(off, Type::usize_type());
                    self.ctx.index(inner_object, idx, inner_object_ty)
                } else {
                    inner_object
                };
                let as_variant =
                    self.ctx.as_variant(new_object, self.ctx.constant_usize(variant_idx));
                let final_object = self.ctx.object(as_variant);
                object_set.insert((final_object, None));
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
