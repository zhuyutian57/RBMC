
use stable_mir::mir::Place;

use crate::expr::expr::*;
use crate::NString;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn claim(&mut self, msg: NString, mut expr: Expr) {
    self.replace_predicates(&mut expr);
    self.rename(&mut expr);
    let mut guard = self.exec_state.cur_state().guard();
    let cond = self.ctx.implies(guard, expr);
    self
      .vc_system
      .borrow_mut()
      .assert(msg, cond);
  }
}