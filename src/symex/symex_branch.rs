use num_bigint::BigInt;
use stable_mir::mir::*;

use super::symex::*;
use crate::{expr::expr::*, program::function::Pc};

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_goto(&mut self, target: &BasicBlockIdx) {
        self.goto(*target, self.ctx._true());
    }

    pub(super) fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
        let discr_expr = self.make_operand(discr);
        let mut otherwise_guard = self.ctx._true();
        for (i, bb) in targets.branches() {
            // branches
            let branch_guard = self.make_branch_guard(discr_expr.clone(), i);
            self.goto(bb, branch_guard.clone());
            otherwise_guard = self.ctx.and(otherwise_guard, self.ctx.not(branch_guard));
        }
        // otherwise
        self.goto(targets.otherwise(), otherwise_guard);
    }

    fn make_branch_guard(&mut self, discr_expr: Expr, i: u128) -> Expr {
        if discr_expr.ty().is_integer() {
            self.ctx
                .eq(discr_expr.clone(), self.ctx.constant_integer(BigInt::from(i), discr_expr.ty()))
        } else if discr_expr.ty().is_bool() {
            if i == 0 { self.ctx.not(discr_expr) } else { discr_expr }
        } else {
            panic!("Not support for this type of SwitchInt")
        }
    }

    /// Register state in state_map
    pub fn goto(&mut self, pc: Pc, mut branch_guard: Expr) {
        self.replace_predicates(&mut branch_guard);
        self.rename(&mut branch_guard);
        branch_guard.simplify();
        if let Some(l) = self.top().cur_loop() {
            let _loop = self.top().function.get_loop(l.0);
            // Not exceed loop bound, keep unwinding.
            // However, if the branch guard is true, the loop stop unwinding.
            if !_loop.contains(&pc) && !self.top().reach_loop_bound(l.0) && !branch_guard.is_true()
            {
                return;
            }
        }
        let mut state = self.top().cur_state.clone();
        state.guard.add(branch_guard);
        if state.guard.is_false() {
            return;
        }
        state.renaming = Some(self.exec_state.renaming.clone());
        self.top_mut().add_state(pc, state);
    }
}
