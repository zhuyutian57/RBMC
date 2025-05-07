use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::symbol::nstring::NString;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_assert(
        &mut self,
        cond: &Operand,
        expected: &bool,
        msg: &AssertMessage,
        target: &usize,
    ) {
        let msg = NString::from("built-in check: ") + msg.description().unwrap();

        let expr = self.make_operand(cond);

        let mut cond = expr.clone();
        self.replace_predicates(&mut cond);
        self.rename(&mut cond);
        // Make assert fail and continue check other assertions
        if *expected == true {
            cond = self.ctx.not(cond);
        }
        self.vc_system.borrow_mut().assert(msg, cond, self.exec_state.cur_span());

        if self.top().pc + 1 == *target {
            self.top_mut().pc += 1;
        } else {
            let state = self.exec_state.cur_state.clone();
            self.cache_unexplored_state(*target, state);
        }
    }
}
