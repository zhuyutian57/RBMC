use core::alloc;
use std::alloc::Layout;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Error;
use std::rc::Rc;

use stable_mir::CrateDef;
use stable_mir::mir::*;
use stable_mir::ty::*;

use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::nstring::NString;
use crate::program::*;
use crate::state::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocKind {
  Alloc,
  Box,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FnKind {
  Allocation(AllocKind, Type),
  Layout(Type),
  AsRef,
  Unwind(FunctionIdx),
}

pub struct Symex<'sym> {
  program : &'sym Program,
  ctx: ExprCtx,
  exec_state: ExecutionState<'sym>,
}

impl<'sym> Symex<'sym> {
  pub fn new(program: &'sym mut Program, ctx: ExprCtx) -> Self {
    Symex {
      program,
      ctx: ctx.clone(),
      exec_state: ExecutionState::new(program, ctx),
    }
  }
  
  pub fn run(&mut self) {
    self.program.show();
    self.exec_state.setup();
    self.symex();
  }

  fn symex(&mut self) {
    while self.exec_state.can_exec() {
      while let Some(pc) = self.symex_frame().cur_pc() {
        // Merge states
        if self.exec_state.merge_states(pc) {
          println!("{:?} - bb{pc}", self.symex_frame().function().name());
          let bb =
            self
            .symex_frame()
            .function()
            .basicblock(pc);
          self.symex_basicblock(bb);
          println!("{:?}", self.symex_frame().cur_state());
        } else {
          self.symex_frame().inc_pc();
        }
      }
      self.exec_state.pop_frame();
    }
  }

  fn symex_frame(&mut self) -> &mut Frame<'sym> {
    self.exec_state.cur_frame()
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

  fn symex_assign(&mut self, place: &Place, rvalue: &Rvalue) {
    // do assignment later
  }

  fn symex_assign_layout(&mut self, place: &Place, ty: Type) {
    let l1_var =
      self
      .exec_state
      .cur_frame()
      .current_local(place.local, false);
    // do assignment later
    let layout = self.ctx.layout(ty);
    self.exec_state.cur_frame().constant_propagate(l1_var.symbol(), layout);
  }

  fn symex_storagelive(&mut self, local: Local) {
    let mut frame = self.symex_frame();
    let decl = frame.function().local_decl(local);
    let var = frame.new_local(local, false);
    if decl.0.is_ptr() {
      assert!(!frame.cur_state().contains_pointer(&var));
      frame.cur_state().add_pointer(var);
    }
  }

  fn symex_storagedead(&mut self, local: Local) {
    let mut frame = self.symex_frame();
    let decl = frame.function().local_decl(local);
    let var = frame.current_local(local, false);
    if decl.0.is_ptr() {
      assert!(frame.cur_state().contains_pointer(&var));
      frame.cur_state().remove_pointer(var);
    }
  }

  fn symex_terminator(&mut self, terminator: &Terminator) {
    match &terminator.kind {
      TerminatorKind::Goto{
        target: target
      } => self.symex_goto(target),
      TerminatorKind::SwitchInt{
        discr: discr,
        targets: targets,
      } => self.symex_switchint(discr, targets),
      TerminatorKind::Drop{
        place: place,
        target: target,
        ..
      } => self.symex_drop(place, target),
      TerminatorKind::Call{
        func: func,
        args: args,
        destination: dest,
        target: target,
        ..
      } => self.symex_call(func, args, dest, target),
      TerminatorKind::Return
        => self.symex_return(),
      _ => {},
    }
  }

  fn symex_goto(&mut self, target: &BasicBlockIdx) {
    let state = self.symex_frame().cur_state().clone();
    self.symex_frame().add_state(*target, state);
    self.symex_frame().inc_pc();
  }

  fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
    for pc in targets.all_targets() {
      let state = self.symex_frame().cur_state().clone();
      // TODO - set path condition
      self.symex_frame().add_state(pc, state);
    }
    self.symex_frame().inc_pc();
  }

  fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
    let state = self.symex_frame().cur_state().clone();
    self.symex_frame().add_state(*target, state);
    self.symex_frame().inc_pc();
  }

  fn allocation_layout(&mut self, arg: &Operand) -> Type {
    match arg {
      Operand::Move(p) => {
        assert!(p.projection.is_empty());
        let mut s =
          self.exec_state.cur_frame().current_local(p.local, false);
        self.exec_state.cur_frame().rename(&mut s, false);
        assert!(s.is_layout());
        Ok(s.layout())
      },
      Operand::Constant(c) => {
        Ok(Type::from(c.ty()))
      }
      _ => Err("Not support"),
    }.expect("Do no exits")
  }

  fn fn_kind(
    &mut self,
    fndef: (FnDef, &GenericArgs),
    args: &Vec<Operand>
  ) -> FnKind {
    let name = NString::from(fndef.0.trimmed_name());
    if self.program.contains_function(name) {
      FnKind::Unwind(self.program.function_idx(name))
    } else if name == NString::from("Layout::new") {
      assert!(fndef.1.0.len() == 1);
      let ty = fndef.1.0[0].ty().expect("Wrong layout type");
      FnKind::Layout(Type::from(*ty))
    } else if name == NString::from("Box::<T>::new") {
      assert!(args.len() == 1);
      let ty = self.allocation_layout(&args[0]);
      FnKind::Allocation(AllocKind::Box, ty)
    } else if name == NString::from("alloc") {
      assert!(args.len() == 1);
      let ty = self.allocation_layout(&args[0]);
      FnKind::Allocation(AllocKind::Alloc, ty)
    } else {
      FnKind::AsRef
    }
  }

  fn symex_call(
    &mut self,
    func: &Operand,
    args: &Vec<Operand>,
    dest: &Place,
    target: &Option<BasicBlockIdx>
  ) {
    let kind =
      self
      .symex_frame()
      .operand_ty(func).kind();
    let fndef = kind.fn_def().expect("Wrong function?");
    let fnkind = self.fn_kind(fndef, args);
    match fnkind {
        FnKind::Unwind(i) => {
          self.exec_state.push_frame(i, args, dest.clone(), *target);
        },
        FnKind::Layout(l) => {
          self.symex_assign_layout(dest, l);
        },
        FnKind::Allocation(k, l) => {
          let object = self.symex_alloc(l);
          // TODO - do projection
          let pt = self.symex_frame().current_local(dest.local, false);
          self.symex_frame().cur_state().points_to_object(pt, object);
          // TODO - do assignment for constant
        },
        FnKind::AsRef => {},
    };
    if matches!(fnkind, FnKind::Unwind(_)) { return; }
    if let Some(t) = target {
      let state = self.symex_frame().cur_state().clone();
      self.symex_frame().add_state(*t, state);
      self.symex_frame().inc_pc();
    }
  }

  fn symex_alloc(&mut self, layout: Type) -> Expr {
    self.exec_state.new_object(layout)
  }

  fn symex_as_ref(&mut self, args: &Vec<Operand>) {
    todo!()
  }

  fn symex_return(&mut self) {
    // TODO: set return value and register state
    // to be merged into stack

    self.symex_frame().inc_pc();
  }

}