use stable_mir::CrateDef;
use stable_mir::mir::mono::Instance;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::value_set::ObjectSet;

/// This mod defines symbolic execution of api in std::ptr

impl<'cfg> Symex<'cfg> {
    pub fn symex_ptr_api(&mut self, instance: Instance, args: Vec<Expr>, dest: Expr) {
        let fty = Type::from(instance.ty());
        let name = NString::from(fty.fn_def().0.trimmed_name());
        if name == "null" || name == "null_mut" {
            self.symex_ptr_null(dest);
        } else if name == "std::ptr::mut_ptr::<impl *mut T>::add" {
            self.symex_ptr_add(dest, args);
        } else if name == "std::ptr::mut_ptr::<impl *mut T>::offset" {
            self.symex_ptr_offset(dest, args);
        } else if name == "std::ptr::mut_ptr::<impl *mut T>::is_null" {
            self.symex_ptr_is_null(dest, args);
        } else {
            panic!("Not support for {name:?}");
        }
    }

    fn symex_ptr_null(&mut self, dest: Expr) {
        let lhs = dest.clone();
        let rhs = self.ctx.null(lhs.ty());
        self.assign(lhs, rhs, self.ctx._true().into());
    }

    fn symex_ptr_add(&mut self, dest: Expr, args: Vec<Expr>) {
        let lhs = dest.clone();

        let pt = args[0].clone();
        let mut count = args[1].clone();
        if count.is_object() {
            count = count.extract_inner_expr();
        }
        let rhs = self.ctx.offset(pt, count);

        self.assign(lhs, rhs, self.ctx._true().into());
    }

    fn symex_ptr_offset(&mut self, dest: Expr, args: Vec<Expr>) {
        let lhs = dest.clone();

        let pt = args[0].clone();
        let mut count = args[1].clone();
        if count.is_object() {
            count = count.extract_inner_expr();
        }
        let rhs = self.ctx.offset(pt, count);

        self.assign(lhs, rhs, self.ctx._true().into());
    }

    fn symex_ptr_is_null(&mut self, dest: Expr, args: Vec<Expr>) {
        let lhs = dest.clone();

        let pt = args[0].clone();
        // Use value_set to optimize
        let mut objects = ObjectSet::new();
        self.exec_state.cur_state.get_value_set(pt.clone(), &mut objects);
        let rhs = if objects.iter().fold(true, |acc, x| acc & !x.0.is_null_object()) {
            // Do not points to NULL object
            self.ctx._false()
        } else if objects.iter().fold(true, |acc, x| acc & x.0.is_null_object()) {
            // Only contains NULL object
            self.ctx._true()
        } else {
            // May be NULL
            self.ctx.eq(pt.clone(), self.ctx.null(pt.ty()))
        };

        self.assign(lhs, rhs, self.ctx._true().into());
    }
}
