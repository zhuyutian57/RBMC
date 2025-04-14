use stable_mir::CrateDef;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::value_set::ObjectSet;

/// This mod defines symbolic execution of api in std::ptr

impl<'cfg> Symex<'cfg> {
    pub fn symex_ptr_api(&mut self, fndef: &FunctionDef, args: Vec<Expr>, dest: Expr) {
        let name = NString::from(fndef.0.trimmed_name());
        if name == "eq" {
            self.symex_ptr_eq(dest, args);
        } else if name == "null_mut" || name == "null" {
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

    fn symex_ptr_eq(&mut self, dest: Expr, args: Vec<Expr>) {
        assert!(args.len() == 2);
        let lhs = dest.clone();

        let p1 = args[0].clone();
        let p2 = args[1].clone();
        let mut rhs = self.ctx.eq(p1, p2);
        self.replace_predicates(&mut rhs);

        self.assign(lhs, rhs, self.ctx._true().into());
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
        self.top().cur_state.get_value_set(pt.clone(), &mut objects);
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
