use num_bigint::BigInt;
use stable_mir::mir::*;

use super::state::State;
use super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::program::function::Pc;
use crate::symbol::nstring::NString;
use crate::symbol::symbol::{Ident, Level};

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_switchint(
        &mut self,
        discr: &Operand,
        targets: &SwitchTargets
    ) -> usize {
        let discr_expr = self.make_operand(discr).unwrap_predicates();
        let mut otherwise_guard = self.ctx._true();
        let mut branches = Vec::new();
        for (i, pc) in targets.branches() {
            let mut branch_guard = self.make_branch_guard(discr_expr.clone(), i);
            self.rename(&mut branch_guard);
            branches.push((pc, branch_guard.clone()));
            otherwise_guard = self.ctx.and(otherwise_guard, self.ctx.not(branch_guard));
        }
        // otherwise
        branches.push((targets.otherwise(), otherwise_guard));
        
        // Goto the first branch and cache states of other branches.
        for (i, (goto_pc, goto_guard)) in branches.iter().enumerate() {
            if i == 0 { continue; }
            let mut state = self.exec_state.cur_state.clone();
            state.guard.add(goto_guard.clone());
            self.cache_unexplored_state(*goto_pc, state);
        }

        let (goto_pc, goto_guard) = (branches[0].0, branches[0].1.clone());
        self.exec_state.cur_state.guard.add(goto_guard);

        goto_pc
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
    pub fn cache_unexplored_state(&mut self, pc: Pc, mut state: State) {
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
