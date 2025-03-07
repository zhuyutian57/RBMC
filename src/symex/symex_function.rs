
use std::fmt::Error;

use stable_mir::mir::*;
use stable_mir::ty::*;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::*;
use crate::NString;
use super::place_state::*;
use super::symex::*;

impl <'cfg> Symex<'cfg> {
  pub(super) fn symex_call(
    &mut self,
    func: &Operand,
    args: &Vec<Operand>,
    dest: &Place,
    target: &Option<BasicBlockIdx>
  ) {
    let ty = self.top_mut().function().operand_type(func);
    let fndef = ty.fn_def();
    let name = NString::from(fndef.0.name());
    let trimmed_name = NString::from(fndef.0.trimmed_name());
    if self.program.contains_function(trimmed_name) {
      let i = self.program.function_idx(trimmed_name);
      self.symex_function(i, args, dest, target);
      return;
    } else if name.contains(NString::from("std::alloc")) {
      self.symex_alloc_api(&fndef, args, dest);
    } else if name.contains(NString::from("std::boxed")) {
      self.symex_boxed_api(&fndef, args, dest);
    } else if name.contains(NString::from("std::ops")) {
      self.symex_ops_api(&fndef, args, dest);
    } else if name.contains(NString::from("std::ptr")) {
      self.symex_ptr_api(&fndef, args, dest);
    } else if name.contains(NString::from("std::vec")) {
      self.symex_vec_api(&fndef, args, dest);
    } else {
      panic!("Do not support {name:?}")
    }

    if let Some(t) = target {
      let state = self.top_mut().cur_state().clone();
      self.register_state(*t, state);
      self.top_mut().inc_pc();
    }
  }

  fn symex_function(
    &mut self,
    i: FunctionIdx,
    args: &Vec<Operand>,
    dest: &Place,
    target: &Option<BasicBlockIdx>
  ) {
    let mut arg_exprs = Vec::new();
    for arg in args {
      arg_exprs.push(self.make_operand(arg));
    }
    // push frame for new name
    self
      .exec_state
      .push_frame(i, Some(dest.clone()), *target);
    // Set arguements
    let args = self.top_mut().function().args();
    if !args.is_empty() {
      for arg_local in args.iter() {
        let lhs = self.exec_state.l0_local(*arg_local);
        let rhs = arg_exprs[*arg_local - 1].clone();
        self.assign(lhs, rhs, self.ctx._true());
      }
    }
    let state = self.top_mut().cur_state().clone();
    self.register_state(0, state);
  }

  pub(super) fn symex_return(&mut self) {
    let n = self.top_mut().function().size();
    let state = self.top_mut().cur_state().clone();
    self.register_state(n, state);

    self.top_mut().inc_pc();
  }

  pub(super) fn symex_end_function(&mut self) {
    let pc = self.top().function().size();
    // Must exist
    assert!(self.merge_states(pc));
    if !self.exec_state.can_exec() { return; }

    let frame = self.exec_state.pop_frame();
    self.top_mut().cur_state = frame.cur_state.clone();
    
    // Assign return value
    if !frame.function().local_type(0).is_unit() {
      if let Some(ret) = &frame.destination {
        let lhs = self.make_project(ret);
        let rhs_ident = frame.local_ident(0);
        let rhs_ty = frame.function().local_type(0);
        let rhs = self.exec_state.l0_symbol(rhs_ident, rhs_ty);
        self.assign(lhs, rhs, self.ctx._true());
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