use stable_mir::CrateDef;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::Guard;
use crate::expr::ty::*;
use crate::symbol::nstring::NString;
use crate::symbol::symbol::Symbol;
use crate::symex::place_state::PlaceState;
use crate::symex::projection::Mode;

/// This mod defines symbolic execution of api in std::vec
/// In our memory model, `vec` is a special pointer that owns
/// an infinite array.

impl<'cfg> Symex<'cfg> {
    pub fn symex_vec_api(&mut self, fndef: &FunctionDef, args: Vec<Expr>, dest: Expr) {
        let name = NString::from(fndef.0.trimmed_name());
        if name == "Vec::<T>::new" {
            self.symex_vec_new(dest);
        } else if name == "Vec::<T, A>::push" {
            self.symex_vec_push(args);
        } else if name == "Vec::<T, A>::pop" {
            self.symex_vec_pop(dest, args);
        } else {
            panic!("Not support for {name:?}");
        }
    }

    fn symex_vec_new(&mut self, dest: Expr) {
        let lhs = dest.clone();
        let object = self.exec_state.new_object(lhs.ty().pointee_ty());

        // Construct vec pointer
        let inner_pt = self.ctx.address_of(object.clone(), object.extract_address_type());
        let len = self.ctx.constant_usize(0);
        let ident = object.extract_inner_expr().extract_symbol().ident();
        let cap_sym = Symbol::from(ident + "_size");
        let cap = self.ctx.mk_symbol(cap_sym, Type::usize_type());
        let _vec = self.ctx._vec(inner_pt, len, cap, dest.ty());
        self.assign(lhs, _vec, self.ctx._true().into());

        // Track new object
        self.track_new_object(object.clone());

        // The newly object is owned by the box pointer
        let place_state = PlaceState::Own;
        self.exec_state.update_place_state(object, place_state);
    }

    fn symex_vec_push(&mut self, args: Vec<Expr>) {
        let guard = Guard::from(self.ctx._true());
        let _vec =
            self.make_deref(args[0].clone(), Mode::Read, guard.clone(), args[0].ty().pointee_ty());
        let value = args[1].clone();

        let inner_pt = self.ctx.inner_pointer(_vec.clone());
        let old_len = self.ctx.vec_len(_vec.clone());
        let len = self.ctx.add(old_len.clone(), self.ctx.constant_usize(1));
        let cap = self.ctx.vec_cap(_vec.clone());

        // Update the inner array
        let inner_array =
            self.make_deref(inner_pt.clone(), Mode::Read, guard.clone(), _vec.ty().pointee_ty());
        let array = self.ctx.object(inner_array);
        let elem_ty = array.ty().elem_type();
        let index = self.ctx.index_non_zero(array, old_len, elem_ty);
        self.assign(index, value, guard.clone());
        // TODO: handle cap

        let lhs = _vec;
        let rhs = self.ctx._vec(inner_pt, len, cap, lhs.ty());
        self.assign(lhs, rhs, guard);
    }

    fn symex_vec_pop(&mut self, dest: Expr, args: Vec<Expr>) {
        let guard = Guard::from(self.ctx._true());
        let _vec =
            self.make_deref(args[0].clone(), Mode::Read, guard.clone(), args[0].ty().pointee_ty());
        let inner_pt = self.ctx.inner_pointer(_vec.clone());
        let inner_array =
            self.make_deref(inner_pt.clone(), Mode::Read, guard.clone(), _vec.ty().pointee_ty());
        let array = self.ctx.object(inner_array);
        let old_len = self.ctx.vec_len(_vec.clone());
        let zero = self.ctx.constant_usize(0);
        let cond = self.ctx.eq(old_len.clone(), zero.clone());
        let sub_one = self.ctx.sub(old_len, self.ctx.constant_usize(1));
        let mut len = self.ctx.ite(cond.clone(), zero, sub_one.clone());
        len.simplify();
        let cap = self.ctx.vec_cap(_vec.clone());

        // Return pop value
        let lhs = dest;
        let none = self.ctx.variant(self.ctx.constant_usize(0), None, lhs.ty());
        let i = sub_one;
        let data = self.ctx.index_non_zero(array.clone(), i, array.ty().elem_type());
        let some = self.ctx.variant(self.ctx.constant_usize(1), Some(data), lhs.ty());
        let rhs = self.ctx.ite(cond, none, some);
        self.assign(lhs, rhs, guard.clone());

        // Update vec
        let lhs = _vec;
        let rhs = self.ctx._vec(inner_pt, len, cap, lhs.ty());
        self.assign(lhs, rhs, guard);
    }
}
