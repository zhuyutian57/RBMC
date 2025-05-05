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
    pub(super) fn symex_goto(&mut self, target: usize) {
        if self.top().pc < target {
            if self.top().pc + 1 == target {
                self.top_mut().pc += 1;
            } else {
                self.cache_unexplored_state(target, self.exec_state.cur_state.clone());
                self.exec_state.reset_to_unexplored_state();
            }
        } else {
            // A back-edge, update the loop unwinding number.
            if self.get_unwind(target) {
                // Loop bound exceed
                self.top_mut().loop_stack.pop();
                let mut cond = self.exec_state.cur_state.guard.to_expr();
                cond = self.ctx.not(cond);
                self.assume(cond);
                self.exec_state.reset_to_unexplored_state();
            } else {
                // Keep unwinding
                self.top_mut().pc = target;
            }
        }
    }

    fn get_unwind(&mut self, pc: Pc) -> bool {
        let (l, count) = self.top().loop_stack.last().unwrap();
        assert!(pc == *l);
        self.config.cli.unwind != 0 && *count >= self.config.cli.unwind    
    }

    pub(super) fn symex_switchint(
        &mut self,
        discr: &Operand,
        targets: &SwitchTargets
    ) {
        let discr_expr = self.make_operand(discr).unwrap_predicates();
        let mut otherwise_guard = self.ctx._true();
        let mut branches = Vec::new();
        for (i, pc) in targets.branches() {
            let mut branch_guard = self.make_branch_guard(discr_expr.clone(), i);
            self.rename(&mut branch_guard);
            branches.push((pc, branch_guard.clone()));
            otherwise_guard = self.ctx.and(otherwise_guard, self.ctx.not(branch_guard.clone()));
        }
        // otherwise
        branches.push((targets.otherwise(), otherwise_guard));

        let mi_branch =
            branches.iter().min_by_key(|&(x, _)| x).unwrap().clone();
        for branch in branches {
            if branch.0 == mi_branch.0 { continue; }
            let mut state = self.exec_state.cur_state.clone();
            state.guard.add(branch.1.clone());
            self.cache_unexplored_state(branch.0, state);
        }
        // Optimization. Stop caching state if the minimal branch pc is the next pc.
        if mi_branch.0 == self.top().pc + 1 {
            self.top_mut().pc += 1;
            self.exec_state.cur_state.guard.add(mi_branch.1.clone());
        } else {
            let mut state = self.exec_state.cur_state.clone();
            state.guard.add(mi_branch.1.clone());
            self.cache_unexplored_state(mi_branch.0, state);
            self.exec_state.reset_to_unexplored_state();
        }
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
}
