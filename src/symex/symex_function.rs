use stable_mir::CrateDef;
use stable_mir::mir::*;

use super::place_state::NPlace;
use super::place_state::PlaceState;
use super::symex::*;
use crate::expr::expr::*;
use crate::program::function::FunctionIdx;
use crate::symbol::nstring::NString;
use crate::symbol::symbol::Level;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_call(
        &mut self,
        func: &Operand,
        args: &Vec<Operand>,
        dest: &Place,
        target: &Option<BasicBlockIdx>,
    ) {
        let ty = self.top_mut().function.operand_type(func);
        let fndef = ty.fn_def();
        let name = NString::from(fndef.0.name());
        let trimmed_name = NString::from(fndef.0.trimmed_name());

        let ret = self.make_project(dest);
        let args_exprs = args.iter().map(|x| self.make_operand(x)).collect::<Vec<_>>();

        if self.program.contains_function(trimmed_name) {
            let i = self.program.function_idx(trimmed_name);
            self.symex_function(i, args, dest, target);
            return;
        } else if name.contains("mirv".into()) {
            self.symex_builtin_function(&fndef, args_exprs.clone(), ret);
        } else if name.contains("std::alloc".into()) {
            self.symex_alloc_api(&fndef, args_exprs.clone(), ret);
        } else if name.contains("std::boxed".into()) {
            self.symex_boxed_api(&fndef, args_exprs.clone(), ret);
        } else if name.contains("std::ops".into()) {
            self.symex_ops_api(&fndef, args_exprs.clone(), ret);
        } else if name.contains("std::ptr".into()) {
            self.symex_ptr_api(&fndef, args_exprs.clone(), ret);
        } else if name.contains("std::vec".into()) {
            self.symex_vec_api(&fndef, args_exprs.clone(), ret);
        } else {
            panic!("Do not support {name:?}")
        }

        // Move semantic
        // for arg_expr in args_exprs {
        //     self.symex_move(arg_expr);
        // }

        if let Some(t) = target {
            self.goto(*t, self.ctx._true());
        } else {
            panic!("Target must exists");
        }
    }

    fn symex_function(
        &mut self,
        i: FunctionIdx,
        args: &Vec<Operand>,
        dest: &Place,
        target: &Option<BasicBlockIdx>,
    ) {
        let mut arg_exprs = Vec::new();
        for arg in args {
            arg_exprs.push(self.make_operand(arg));
        }
        // Push frame for new name
        self.exec_state.push_frame(i, Some(dest.clone()), *target);
        // Set alive local place state
        for local in self.top().function.locals_alive() {
            let l1_local = self.exec_state.current_local(*local, Level::Level1);
            let nplace = NPlace(l1_local.extract_symbol().l1_name());
            self.top_mut().cur_state.update_place_state(nplace, PlaceState::Own);
        }
        // Set arguements
        let args = self.top_mut().function.args();
        if !args.is_empty() {
            for arg_local in args.iter() {
                let lhs = self.exec_state.l0_local(*arg_local);
                let rhs = arg_exprs[*arg_local - 1].clone();
                self.assign(lhs, rhs, self.ctx._true().into());
            }
        }
        self.goto(0, self.ctx._true());
    }

    pub(super) fn symex_return(&mut self) {
        let n = self.top_mut().function.size();
        self.goto(n, self.ctx._true());
    }

    pub(super) fn symex_end_function(&mut self) {
        let pc = self.top().function.size();
        // Must exist
        assert!(self.merge_states(pc));
        if !self.exec_state.can_exec() {
            return;
        }

        let frame = self.exec_state.pop_frame();
        self.top_mut().cur_state = frame.cur_state.clone();

        // Assign return value
        if !frame.function.local_type(0).is_unit() {
            if let Some(ret) = &frame.destination {
                let lhs = self.make_project(ret);
                let rhs_ident = frame.local_ident(0);
                let rhs_ty = frame.function.local_type(0);
                let rhs = self.exec_state.l0_symbol(rhs_ident, rhs_ty);
                self.assign(lhs, rhs, self.ctx._true().into());
            }
        }

        if let Some(t) = &frame.target {
            let mut state = self.top().cur_state.clone();
            state.remove_stack_places(frame.function_id());
            self.top_mut().add_state(*t, state);
        }

        // clear namspace
        self.exec_state.ns.clear_local_symbols(frame.function_id());

        // clear renaming
        self.exec_state.renaming.borrow_mut().cleanr_locals(frame.function_id());

        self.top_mut().inc_pc();
    }
}
