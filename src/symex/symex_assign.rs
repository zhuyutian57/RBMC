
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::symbol::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_assign(&mut self, place: &Place, rvalue: &Rvalue) {
    // construct lhs expr and rhs expr from MIR
    let lhs = self.make_project(place);
    let rhs = self.make_rvalue(rvalue);
    self.assign(lhs, rhs, self.ctx._true());
  }

  pub(super) fn symex_assign_layout(&mut self, place: &Place, ty: Type) {
    // Use l2 symbol to do assignment
    let l2_var = self.make_project(place);
    let layout = self.ctx.mk_type(ty);
    self.assign(l2_var, layout, self.ctx._true());
  }

  pub(super) fn assign(&mut self, lhs: Expr, rhs: Expr, guard: Expr) {
    assert!(lhs.ty().is_layout() || lhs.ty() == rhs.ty());
    // TODO: do more jobs
    self.assign_rec(lhs, rhs, guard);
  }

  fn assign_symbol(&mut self, mut lhs: Expr, mut rhs: Expr, guard: Expr) {
    assert!(lhs.is_symbol());
    
    if !guard.is_true() {
      rhs = self.ctx.ite(guard, rhs, lhs.clone());
    }

    // Rename to l2 rhs
    self.rename(&mut rhs);
    // New l2 symbol
    lhs = self.exec_state.new_symbol(&lhs, Level::Level2);

    self.exec_state.assign(lhs.clone(), rhs.clone());

    if rhs.is_type() { return; }

    // Build VC system
    self.vc_system.borrow_mut().assign(lhs, rhs);
  }

  fn assign_rec(&mut self, lhs: Expr, rhs: Expr, guard: Expr) {
    if lhs.is_symbol() {
      self.assign_symbol(lhs, rhs, guard);
      return;
    }

    if lhs.is_object() {
      let new_lhs = lhs.extract_inner_expr();
      self.assign_rec(new_lhs, rhs, guard);
      return;
    }

    if lhs.is_ite() {
      let sub_exprs = lhs.sub_exprs().unwrap();
      let cond = sub_exprs[0].clone();
      let true_value = sub_exprs[1].clone();
      let false_value = sub_exprs[2].clone();
      
      let mut true_guard = self.ctx.and(guard.clone(), cond.clone());
      true_guard.simplify();
      self.assign_rec(true_value, rhs.clone(), true_guard);
      
      let mut false_guard =
        self.ctx.and(
          guard.clone(),
          self.ctx.not(cond.clone())
        );
      false_guard.simplify();
      self.assign_rec(false_value, rhs.clone(), false_guard);

      return;
    }

    if lhs.is_index() {
      let new_lhs = lhs.extract_object();
      let index = lhs.extract_index();
      let new_rhs = self.ctx.store(new_lhs.clone(), index, rhs.clone());
      self.assign_rec(new_lhs, new_rhs, guard);
      return;
    }

    panic!("Do not support assignment:\n{lhs:?} = {rhs:?}");
  }
}