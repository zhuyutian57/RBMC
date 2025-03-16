
use stable_mir::mir::*;
use stable_mir::CrateDef;

use crate::expr::context::*;
use crate::expr::constant::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::program::program::*;
use crate::symbol::symbol::*;
use crate::vc::vc::*;
use crate::config::config::Config;
use super::exec_state::*;
use super::frame::*;
use super::place_state::*;
use super::state::State;

pub struct Symex<'cfg> {
  pub(super) config : &'cfg Config,
  pub(super) program: &'cfg Program,
  pub(super) ctx: ExprCtx,
  pub(super) exec_state: ExecutionState<'cfg>,
  pub(super) vc_system: VCSysPtr,
}

impl<'cfg> Symex<'cfg> {
  pub fn new(
    config: &'cfg Config,
    vc_system: VCSysPtr) -> Self {
    let ctx = config.expr_ctx.clone();
    let mut exec_state =
      ExecutionState::new(&config.program, ctx.clone());
    exec_state.setup();

    let mut symex =
      Symex {
        config,
        program: &config.program,
        ctx: ctx.clone(),
        exec_state,
        vc_system
      };
    symex.init();
    symex
  }

  fn init(&mut self) {
    // Init static variable
    for def in self.program.static_variables() {
      let name = NString::from(def.trimmed_name());
      let ty = Type::from(def.ty());
      let symbol = self.exec_state.l0_symbol(name, ty);
      let object = self.ctx.object(symbol);
      self.exec_state.ns.insert_object(object.clone());
      
      let init_value =
        match def.eval_initializer() {
          Ok(allocation)
            => self.make_constant_from_allocation(&allocation, ty),
          _ => panic!("Some thing wrong?"),
        };
      self.assign(object, init_value, self.ctx._true().into());
    }

    // Global varialbes for Encoding
    let alloc_array =
      self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
    let const_array =
      self.ctx.constant_array(self.ctx.constant_bool(false), None);
    self.assign(alloc_array, const_array, self.ctx._true().into());
  }

  pub fn run(&mut self) {
    while self.exec_state.can_exec() { self.symex(); }
    self.memory_leak_check();
  }

  fn symex(&mut self) {
    while let Some(pc) = self.top_mut().cur_pc() {
      // Merge states
      if self.merge_states(pc) {
        if self.config.cli.enable_display_state_bb() {
          println!(
            "Enter {:?} - bb{pc}\n{:?}",
            self.top_mut().function().name(),
            self.top_mut().cur_state()
          );
        }
        let bb = self.top_mut().function().basicblock(pc);
        self.symex_basicblock(bb);
      } else {
        self.top_mut().inc_pc();
      }
    }
    self.symex_end_function();
  }

  pub(super) fn top(&self) -> &Frame<'cfg> {
    self.exec_state.top()
  }

  pub(super) fn top_mut(&mut self) -> &mut Frame<'cfg> {
    self.exec_state.top_mut()
  }

  pub(super) fn register_state(&mut self, pc: Pc, mut state: State) {
    state.renaming = Some(self.exec_state.renaming.clone());
    self.top_mut().add_state(pc, state);
  }

  fn symex_basicblock(&mut self, bb: &BasicBlock) {
    for (i, statement) 
      in bb.statements.iter().enumerate() {
      self.symex_statement(statement);
      if self.config.cli.enable_display_state_statement() {
        println!(
          "After symex {i}\n{:?}",
          self.top_mut().cur_state()
        );
      }
    }
    self.symex_terminator(&bb.terminator);
    if self.config.cli.enable_display_state_terminator() {
      println!(
        "After symex terminator\n{:?}",
        self.top_mut().cur_state()
      );
    }
  }

  fn symex_statement(&mut self, statement: &Statement) {
    match &statement.kind {
      StatementKind::Assign(place, rvalue)
        => self.symex_assign(place, rvalue),
      StatementKind::StorageLive(local) 
        => self.symex_storagelive(*local),
      StatementKind::StorageDead(local) 
        => self.symex_storagedead(*local),
      _ => {},
    }
  }

  fn symex_storagelive(&mut self, local: Local) {
    // Set a new l1 local variable
    self.exec_state.new_local(local, Level::Level1);
  }

  fn symex_storagedead(&mut self, local: Local) {
    let var = self.exec_state.current_local(local, Level::Level1);
    let move_expr = self.ctx._move(self.ctx.object(var));
    // Remove the pointer in value set and set a fresh value
    self.symex_move(move_expr);
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
      TerminatorKind::Assert { 
        cond,
        expected,
        msg,
        target,
        ..
      } => self.symex_assert(cond, expected, msg, target),
      _ => {},
    }
  }
}