use num_bigint::BigInt;
use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::program::function::Pc;
use crate::symbol::nstring::NString;
use crate::symbol::symbol::{Ident, Level};

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_goto(&mut self, target: &BasicBlockIdx) {
        self.goto(*target, self.ctx._true());
    }

    pub(super) fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
        let discr_expr = self.make_operand(discr).unwrap_predicates();
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
            let mut discr = self.ctx.eq(
                discr_expr.clone(),
                self.ctx.constant_integer(BigInt::from(i), discr_expr.ty()),
            );
            self.rename(&mut discr);
            discr.simplify();
            if discr.is_constant() || discr.is_symbol() {
                return discr;
            }

            let ident = Ident::Global(NString::SYMEX_GUARD);
            let l0_guard_ident = self.exec_state.l0_symbol(ident, Type::bool_type());
            let guard_ident = self.exec_state.new_symbol(&l0_guard_ident, Level::Level1);
            self.assign(guard_ident.clone(), discr, self.ctx._true().into());
            guard_ident
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

        let mut state = self.top().cur_state.clone();
        state.guard.add(branch_guard);
        if state.guard.is_false() {
            return;
        }

        state.renaming = Some(self.exec_state.renaming.clone());
        self.top_mut().add_state(pc, state);
    }

    // fn ask_rts(&mut self, expr: Expr) -> PathFeasibility {
    //     // Push all formulas that is not in solver
    //     let mut i = &mut self.runtime.0;
    //     while *i < self.vc_system.borrow().size() {
    //         match self.vc_system.borrow().nth(*i).kind {
    //             VcKind::Assign(lhs, rhs)
    //                 => self.runtime.1.assert_assign(lhs, rhs),
    //                 _ => {},
    //             VcKind::Assume(expr)
    //                 => self.runtime.1.assert_expr(expr),
    //         }
    //         *i += 1;
    //     }

    //     let res1;
    //     self.runtime.1.push();
    //     self.runtime.1.assert_expr(expr.clone());
    //     res1 = self.runtime.1.check();
    //     self.runtime.1.pop();

    //     let res2;
    //     self.runtime.1.push();
    //     self.runtime.1.assert_expr(self.ctx.not(expr.clone()));
    //     res2 = self.runtime.1.check();
    //     self.runtime.1.pop();

    //     let res = if res1 == PResult::PSat && res1 == PResult::PSat {
    //         PathFeasibility::PATHUnkown
    //     } else if res1 == PResult::PSat && res2 == PResult::PUnsat {
    //         PathFeasibility::PATHTrue
    //     } else if res1 == PResult::PUnsat && res2 == PResult::PSat {
    //         PathFeasibility::PATHFalse
    //     } else {
    //         PathFeasibility::PATHImpissible
    //     };

    //     res
    // }
}
