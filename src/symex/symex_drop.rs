
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::NString;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  
  pub(super) fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
    let state = self.top().cur_state().clone();

    // Drop recursively
    let object = self.make_project(place);
    self.symex_drop_rec(object, self.ctx.constant_bool(true));

    self.register_state(*target, state);
    self.top().inc_pc();
  }

  pub(super) fn symex_drop_rec(&mut self, expr: Expr, guard: Expr) {
    if expr.is_object() {
      if expr.ty().is_box() {
        // TODO: do dereference and add assertion

        let pointer_ident = self.ctx.pointer_ident(expr.extract_inner_expr());      
        let alloc_array_sym = self.exec_state.ns.lookup(NString::ALLOC_SYM);
        let alloc_array = self.ctx.object(alloc_array_sym, Ownership::Own);
        let index =
          self.ctx.index(alloc_array, pointer_ident, Type::bool_type());
        self.assign_rec(index, self.ctx.constant_bool(false), guard.clone());
      } else {
        panic!("drop {:?} should be implemented", expr.ty());
      }
      return;
    }

    if expr.is_ite() {
      let cond = expr.extract_cond();
      let true_value = expr.extract_true_value();
      let false_value = expr.extract_false_value();

      let true_guard =
        self.ctx.and(guard.clone(), cond.clone());
      let false_guard = 
        self.ctx.and(guard.clone(), self.ctx.not(cond));

      self.symex_drop_rec(true_value, true_guard);
      self.symex_drop_rec(false_value, false_guard);
      return;
    }

    panic!("Not implement drop {:?}", expr.ty());
  }
}