use num_bigint::BigInt;
use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_goto(&mut self, target: &BasicBlockIdx) {
        let state = self.top_mut().cur_state().clone();
        self.register_state(*target, state);
        self.top_mut().inc_pc();
    }

    pub(super) fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
        let mut discr_expr = self.make_operand(discr);
        self.replace_predicates(&mut discr_expr);
        let mut otherwise_guard = self.ctx._true();
        for (i, bb) in targets.branches() {
            let mut state = self.top_mut().cur_state().clone();
            // branches
            let mut branch_guard = self.make_branch_guard(discr_expr.clone(), i);
            self.rename(&mut branch_guard);
            state.guard.add(branch_guard.clone());
            self.register_state(bb, state.clone());
            otherwise_guard = self.ctx.and(otherwise_guard, self.ctx.not(branch_guard));
        }
        // otherwise
        otherwise_guard.simplify();
        let mut otherwise_state = self.top_mut().cur_state().clone();
        otherwise_state.guard.add(otherwise_guard);
        self.register_state(targets.otherwise(), otherwise_state);

        self.top_mut().inc_pc();
    }

    fn make_branch_guard(&mut self, discr_expr: Expr, i: u128) -> Expr {
        if discr_expr.ty().is_integer() {
            self.ctx
                .eq(discr_expr.clone(), self.ctx.constant_integer(BigInt::ZERO, discr_expr.ty()))
        } else if discr_expr.ty().is_bool() {
            if i == 0 { self.ctx.not(discr_expr) } else { discr_expr }
        } else {
            panic!("Not support for this type of SwitchInt")
        }
    }
}
