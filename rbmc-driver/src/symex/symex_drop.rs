use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::*;
use crate::symbol::nstring::NString;
use crate::symex::projection::Mode;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
        // Drop recursively
        let place = self.make_project(place);
        let object = self.ctx.object(place);
        self.symex_drop_rec(object, self.ctx._true().into());

        self.goto(*target, self.ctx._true());
    }

    fn symex_drop_rec(&mut self, expr: Expr, guard: Guard) {
        if expr.is_object() {
            if expr.ty().is_box() {
                self.drop_box(expr.clone(), guard.clone());
            } else if expr.ty().is_vec() {
                self.drop_vec(expr.clone(), guard.clone());
            } else if expr.ty().is_struct() {
                self.drop_struct(expr.clone(), guard.clone());
            } else {
                todo!("drop {:?}", expr.ty());
            }
            return;
        }

        if expr.is_ite() {
            let cond = expr.extract_cond();
            let true_value = expr.extract_true_value();
            let false_value = expr.extract_false_value();

            let mut true_cond = cond.clone();
            self.rename(&mut true_cond);
            let true_guard = Guard::from(true_cond);
            let mut false_cond = self.ctx.not(cond);
            self.rename(&mut false_cond);
            let false_guard = Guard::from(false_cond);

            self.symex_drop_rec(true_value, true_guard);
            self.symex_drop_rec(false_value, false_guard);
            return;
        }

        panic!("Not implement drop {:?}", expr.ty());
    }

    /// Drop a box will free the memory it points to
    fn drop_box(&mut self, _box: Expr, guard: Guard) {
        // Check whethe the box is uninitilized
        self.make_deref(_box.clone(), Mode::Drop, guard.clone(), _box.ty().pointee_ty());
        self.top_mut().cur_state.dealloc_objects(_box.clone());
        self.top_mut().cur_state.remove_pointer(_box.clone());

        let pointer_base = self.ctx.pointer_base(_box);
        let alloc_array = self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
        let index = self.ctx.index(alloc_array, pointer_base, Type::bool_type());
        self.assign(index, self.ctx._false(), guard.clone());
    }

    fn drop_vec(&mut self, _vec: Expr, guard: Guard) {
        // Check whethe the vec is uninitilized
        self.make_deref(_vec.clone(), Mode::Drop, guard.clone(), _vec.ty().pointee_ty());
        self.top_mut().cur_state.dealloc_objects(_vec.clone());
        self.top_mut().cur_state.remove_pointer(_vec.clone());

        let pointer_base = self.ctx.pointer_base(_vec);
        let alloc_array = self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
        let index = self.ctx.index(alloc_array, pointer_base, Type::bool_type());
        self.assign(index, self.ctx._false(), guard.clone());
    }

    /// Drop a struct may drop the inner box pointer
    fn drop_struct(&mut self, st: Expr, guard: Guard) {
        let def = st.ty().struct_def();
        for (i, (_, ty)) in def.1.iter().enumerate() {
            if !ty.is_box() && !ty.is_struct() {
                continue;
            }
            let object = self.ctx.object(self.ctx.index(
                st.clone(),
                self.ctx.constant_isize(i as isize),
                *ty,
            ));
            self.symex_drop_rec(object, guard.clone());
        }
    }
}
