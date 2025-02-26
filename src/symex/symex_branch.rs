
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
    if discr_expr.ty().is_bool() {
      let mut true_state = self.top().cur_state().clone();
      true_state.guard =
        self.ctx.and(true_state.guard(), discr_expr.clone());
      self.exec_state.rename(&mut true_state.guard, Level::Level2);
      let true_branch = targets.all_targets()[0];
      self.register_state(true_branch, true_state);

      let mut false_state = self.top().cur_state().clone();
      false_state.guard =
        self.ctx.and(
          false_state.guard.clone(),
          self.ctx.not(discr_expr.clone())
        );
      self.exec_state.rename(&mut false_state.guard, Level::Level2);
      let false_branch = targets.all_targets()[1];
      self.register_state(false_branch, false_state);
    } else if discr_expr.ty().is_integer() {
      let mut state = self.top().cur_state().clone();
      let state_guard = state.guard();
      let mut otherwise_guard = state.guard();
      // branches
      for (i, bb) in targets.branches() {
        let branch_guard =
          self.ctx.eq(
            discr_expr.clone(),
            self.ctx.constant_integer(BigInt(false, i), discr_expr.ty())
          );
        state.guard =
          self.ctx.and(state_guard.clone(), branch_guard.clone());
        self.exec_state.rename(&mut state.guard, Level::Level2);
        self.register_state(bb, state.clone());
        otherwise_guard = 
          self.ctx.and(
            otherwise_guard,
            self.ctx.not(branch_guard)
          );
      }
      // otherwise
      state.guard = otherwise_guard;
      self.register_state(targets.otherwise(), state);
    } else {
      panic!("Not implement {discr:?}");
    }

    self.top().inc_pc();
  }
}