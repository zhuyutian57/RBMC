
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::constant::*;
use crate::symbol::symbol::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_goto(&mut self, target: &BasicBlockIdx) {
    let state = self.top().cur_state().clone();
    self.register_state(*target, state);
    self.top().inc_pc();
  }

  pub(super) fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
    let discr_expr = self.make_operand(discr);
    let mut otherwise_guard = self.ctx.constant_bool(true);
    for (i, bb) in targets.branches() {
      let mut state = self.top().cur_state().clone();
      // branches
      let branch_guard = self.make_branch_guard(discr_expr.clone(), i);
      state.guard =
        self.ctx.and(state.guard(), branch_guard.clone());
      state.guard.simplify();
      self.rename(&mut state.guard);
      self.register_state(bb, state.clone());
      otherwise_guard = 
        self.ctx.and(
          otherwise_guard,
          self.ctx.not(branch_guard)
        );
    }
    // otherwise
    let mut otherwise_state = self.top().cur_state().clone();
    otherwise_state.guard =
      self.ctx.and(otherwise_state.guard(), otherwise_guard);
    otherwise_state.guard.simplify();
    self.rename(&mut otherwise_state.guard);
    self.register_state(targets.otherwise(), otherwise_state);

    self.top().inc_pc();
  }

  fn make_branch_guard(&mut self, discr_expr: Expr, i: u128) -> Expr {
    if discr_expr.ty().is_integer() {
      self.ctx.eq(
        discr_expr.clone(),
        self.ctx.constant_integer(BigInt(false, i), discr_expr.ty())
      )
    } else if discr_expr.ty().is_bool() {
      if i == 0 {
        self.ctx.not(discr_expr)
      } else {
        discr_expr
      }
    } else {
      panic!("Not support for this type of SwitchInt")
    }
  }

}