
use std::collections::HashMap;
use std::fmt::Error;

use stable_mir::CrateDef;
use stable_mir::mir::*;
use stable_mir::target::*;
use stable_mir::ty::*;

use crate::expr::context::*;
use crate::expr::constant::*;
use crate::expr::expr::*;
use crate::expr::op::*;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::solvers::solver::Solver;
use crate::symbol::nstring::*;
use crate::program::program::*;
use crate::symbol::symbol::*;
use crate::vc::vc::*;
use crate::config::config::Config;
use super::exec_state::*;
use super::frame::*;
use super::place_state::*;
use super::projection::*;
use super::state::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AllocKind {
  Alloc,
  Box,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FnKind {
  Unwind(FunctionIdx),
  Layout(Type),
  Allocation(AllocKind, Type),
  AsMut(Operand),
  AsRef(Operand),
}

pub struct Symex<'cfg> {
  pub(super) program : &'cfg Program,
  pub(super) ctx: ExprCtx,
  pub(super) exec_state: ExecutionState<'cfg>,
  pub(super) vc_system: VCSysPtr,
}

impl<'cfg> Symex<'cfg> {
  pub fn new(
    program: &'cfg Program,
    ctx: ExprCtx,
    vc_system: VCSysPtr) -> Self {
    let mut exec_state = ExecutionState::new(program, ctx.clone());
    exec_state.setup();

    let mut symex =
      Symex { program, ctx: ctx.clone(), exec_state, vc_system };
    let mut alloc_array =
      symex.exec_state.ns.lookup_object(NString::ALLOC_SYM);
    let mut const_array =
      ctx.constant_array(Constant::Bool(false), Type::bool_type());
    symex.assign(alloc_array, const_array, ctx.constant_bool(true));
    symex
  }

  pub fn can_exec(&self) -> bool { self.exec_state.can_exec() }

  pub(super) fn top(&mut self) -> &mut Frame<'cfg> {
    self.exec_state.top_mut()
  }

  pub(super) fn register_state(&mut self, pc: Pc, mut state: State) {
    state.renaming = Some(Box::new(self.exec_state.renaming.clone()));
    self.top().add_state(pc, state);
  }

  pub fn symex(&mut self) {
    while let Some(pc) = self.top().cur_pc() {
      // Merge states
      if self.merge_states(pc) {
        println!(
          "Enter {:?} - bb{pc}\n{:?}",
          self.top().function().name(),
          self.top().cur_state()
        );
        let bb = self.top().function().basicblock(pc);
        self.symex_basicblock(bb);
      } else {
        self.top().inc_pc();
      }
    }
    self.exec_state.pop_frame();
  }

  fn symex_basicblock(&mut self, bb: &BasicBlock) {
    for statement in bb.statements.iter() {
      self.symex_statement(statement);
    }
    self.symex_terminator(&bb.terminator);
  }

  fn symex_statement(&mut self, statement: &Statement) {
    match &statement.kind {
      StatementKind::Assign(place, rvalue) => {
        self.symex_assign(place, rvalue);
      },
      StatementKind::StorageLive(local) => {
        self.symex_storagelive(*local);
      },
      StatementKind::StorageDead(local) => {
        self.symex_storagedead(*local);
      },
      _ => {},
    }
  }

  fn symex_storagelive(&mut self, local: Local) {
    let var = self.exec_state.new_local(local, Level::Level1);
    // TODO: maybe do something will pointers
  }

  fn symex_storagedead(&mut self, local: Local) {
    let var = self.exec_state.new_local(local, Level::Level1);
    if var.ty().is_any_ptr() {
      self.exec_state.cur_state_mut().remove_pointer(var);
    }
  }

  fn symex_terminator(&mut self, terminator: &Terminator) {
    match &terminator.kind {
      TerminatorKind::Goto{ target}
        => self.symex_goto(target),
      TerminatorKind::SwitchInt{discr, targets }
        => self.symex_switchint(discr, targets),
      TerminatorKind::Drop{ place, target, ..}
        => self.symex_drop(place, target),
      TerminatorKind::Call{
        func,
        args,
        destination,
        target,
        ..
      } => self.symex_call(func, args, destination, target),
      TerminatorKind::Return
        => self.symex_return(),
      _ => {},
    }
  }
}