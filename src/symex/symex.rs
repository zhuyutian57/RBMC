use std::fmt::Error;

use stable_mir::CrateDef;
use stable_mir::mir::*;
use stable_mir::target::*;
use stable_mir::ty::*;

use crate::expr::constant::Constant;
use crate::expr::op::BinOp;
use crate::expr::op::UnOp;
use crate::expr::{context::*, expr::*, ty::*};
use crate::symbol::nstring::NString;
use crate::program::program::*;
use crate::symbol::symbol::*;
use crate::vc::vc::*;
use super::exec_state::*;
use super::frame::*;
use super::projection::*;

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
  vc_system: VCSystem,
}

impl<'sym> Symex<'sym> {
  pub fn new(program: &'sym mut Program, ctx: ExprCtx) -> Self {
    Symex {
      program,
      ctx: ctx.clone(),
      exec_state: ExecutionState::new(program, ctx),
      vc_system: VCSystem::default(),
    }
  }
  
  pub fn run(&mut self) {
    self.program.show();
    self.exec_state.setup();
    self.symex();
  }

  fn symex(&mut self) {
    while self.exec_state.can_exec() {
      while let Some(pc) = self.cur_frame().cur_pc() {
        // Merge states
        if self.exec_state.merge_states(pc) {
          println!(
            "Enter {:?} - bb{pc}\n{:?}",
            self.cur_frame().function().name(),
            self.cur_frame().cur_state()
          );
          let bb = self.cur_frame().function().basicblock(pc);
          self.symex_basicblock(bb);
        } else {
          self.cur_frame().inc_pc();
        }
      }
      self.exec_state.pop_frame();
    }
  }

  fn cur_frame(&mut self) -> &mut Frame<'sym> {
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

  /// Interface to do projection(dereference)
  fn make_project(&mut self, place: &Place) -> Expr {
    let local =
      self
        .cur_frame()
        .current_local(place.local, Level::level1);

    if place.projection.is_empty() { return local; }
    
    let mut projector = Projector::new(self.cur_frame());
    projector.project(place)
  }

  fn make_mirconst(&mut self, mirconst: &MirConst) -> Expr {
    match mirconst.kind() {
      ConstantKind::Allocated(allocation) => {
        let ty = Type::from(mirconst.ty());
        let fields = ty.variant();
        let mut value_vec = Vec::new();
        let bytes = &allocation.bytes;
        for i in 0..fields.len() {
          let l = 
            if MachineInfo::target().endian == Endian::Little {
              bytes.len() - allocation.align as usize * (i + 1)
            } else {
              allocation.align as usize * i
            };
          let r = l + allocation.align as usize;
          let mut raw_bytes = Vec::new();
          for j in l..r {
            if let Some(x) = bytes[j] {
              raw_bytes.push(x);
            }
          }
          if fields[i].is_bool() {
            assert!(raw_bytes.len() == 1);
            value_vec.push(Constant::Bool(raw_bytes[0] == 1));
            continue;
          }
          let (sign, value) =
            read_target_integer(
              raw_bytes.as_slice(),
              fields[i].is_signed()
            );
          value_vec.push(Constant::Integer(sign, value));
        }

        Ok(self.ctx.constant_struct(value_vec, ty))
      }
      _ => Err("Not support"),
    }.expect("Not support")
  }

  fn make_operand(&mut self, operand: &Operand) -> Expr {
    match operand {
      Operand::Copy(p) => {
        // TODO: handle copy semantic?
        self.make_project(p)
      },
      Operand::Move(p) => {
        // TODO: handle move semantic?
        self.make_project(p)
      },
      Operand::Constant(op) 
        => self.make_mirconst(&op.const_),
    }
  }

  /// Create l1 formula from Rvalue(MIR)
  fn make_rvalue(&mut self, rvalue: &Rvalue) -> Expr {
    let ty = self.cur_frame().function().rvalue_type(rvalue);
    match rvalue {
      Rvalue::AddressOf(m, p) => {
        let place = self.make_project(p);
        let address_of = self.ctx.address_of(place, ty);
        Ok(address_of)
      },
      Rvalue::BinaryOp(mir_op, lop, rop) => {
        let op = BinOp::from(mir_op.clone());
        let lhs = self.make_operand(lop);
        let rhs = self.make_operand(rop);
        let expr =
          match op {
            BinOp::Add => self.ctx.add(lhs, rhs),
            BinOp::Sub => self.ctx.sub(lhs, rhs),
            BinOp::Mul => self.ctx.mul(lhs, rhs),
            BinOp::Div => self.ctx.div(lhs, rhs),
            BinOp::Eq => self.ctx.eq(lhs, rhs),
            BinOp::Ne => self.ctx.ne(lhs, rhs),
            BinOp::Ge => self.ctx.ge(lhs, rhs),
            BinOp::Gt => self.ctx.gt(lhs, rhs),
            BinOp::Le => self.ctx.le(lhs, rhs),
            BinOp::Lt => self.ctx.lt(lhs, rhs),
            BinOp::And => self.ctx.and(lhs, rhs),
            BinOp::Or => self.ctx.or(lhs, rhs),
          };
        Ok(expr)
      },
      Rvalue::UnaryOp(mir_op, o) => {
        let op = UnOp::from(mir_op.clone());
        let operand = self.make_operand(o);
        let expr =
          match op {
            UnOp::Not => self.ctx.not(operand),
            UnOp::Neg => self.ctx.neg(operand),
          };
        Ok(expr)
      },
      Rvalue::Cast(k, operand, t) => {
        todo!();
      },
      Rvalue::CopyForDeref(p) => {
        todo!();
      },
      Rvalue::Ref(_, k, p) => {
        let place = self.make_project(p);
        // TODO: handle borrow kind.
        let address_of = self.ctx.address_of(place, ty);
        Ok(address_of)
      },
      Rvalue::Use(operand)
        => Ok(self.make_operand(operand)),
      _ => Err(Error),
    }.expect("Not support")
  }

  fn symex_assign(&mut self, place: &Place, rvalue: &Rvalue) {
    // construct lhs expr and rhs expr from MIR
    let lhs = self.make_project(place);
    let rhs = self.make_rvalue(rvalue); 
    self.do_assignment(lhs, rhs);
  }

  fn symex_assign_layout(&mut self, place: &Place, ty: Type) {
    // Use l2 symbol to do assignment
    let l2_var =
      self
        .cur_frame()
        .current_local(place.local, Level::level2);
    let layout = self.ctx.layout(ty);
    self.do_assignment(l2_var, layout);
  }

  fn do_assignment(&mut self, mut lhs: Expr, mut rhs: Expr) {
    assert!(lhs.is_symbol());
    
    // New l2 symbol
    lhs = self.cur_frame().new_symbol(&lhs, Level::level2);
    // Rename to l2 rhs
    self.cur_frame().rename(&mut rhs, Level::level2);

    self.cur_frame().assignment(lhs.clone(), rhs.clone());

    if rhs.is_layout() { return; }

    // Build VC system
    self.vc_system.assign(lhs, rhs);
  }

  fn symex_storagelive(&mut self, local: Local) {
    let frame = self.cur_frame();
    let var = frame.new_local(local, Level::level1);
    if var.ty().is_any_ptr() {
      frame.cur_state().add_pointer(var);
    }
  }

  fn symex_storagedead(&mut self, local: Local) {
    let frame = self.cur_frame();
    let var = frame.current_local(local, Level::level1);
    if var.ty().is_any_ptr() {
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
    let state = self.cur_frame().cur_state().clone();
    self.cur_frame().add_state(*target, state);
    self.cur_frame().inc_pc();
  }

  fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
    for pc in targets.all_targets() {
      let state = self.cur_frame().cur_state().clone();
      // TODO - set path condition
      self.cur_frame().add_state(pc, state);
    }
    self.cur_frame().inc_pc();
  }

  fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
    let state = self.cur_frame().cur_state().clone();
    // TODO: exec drop
    self.cur_frame().add_state(*target, state);
    self.cur_frame().inc_pc();
  }

  fn make_layout(&mut self, arg: &Operand) -> Type {
    match arg {
      Operand::Move(p) => {
        assert!(p.projection.is_empty());
        let mut s =
          self.exec_state.cur_frame().current_local(p.local, Level::level2);
          
        self.exec_state.cur_frame().rename(&mut s, Level::level2);
        assert!(s.is_layout());
        Ok(s.layout())
      },
      Operand::Constant(c) => {
        Ok(Type::from(c.ty()))
      }
      _ => Err(Error),
    }.expect("Do no exits")
  }

  fn make_fn_kind(
    &mut self,
    fndef: (FnDef, GenericArgs),
    args: &Vec<Operand>
  ) -> FnKind {
    let name = NString::from(fndef.0.trimmed_name());
    if self.program.contains_function(name) {
      FnKind::Unwind(self.program.function_idx(name))
    } else if name == NString::from("Layout::new") {
      assert!(fndef.1.0.len() == 1);
      let ty = fndef.1.0[0].ty().unwrap();
      FnKind::Layout(Type::from(*ty))
    } else if name == NString::from("Box::<T>::new") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
      FnKind::Allocation(AllocKind::Box, ty)
    } else if name == NString::from("alloc") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
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
    let ty = self.cur_frame().function().operand_type(func);
    let fndef = ty.fn_def();
    let fnkind = self.make_fn_kind(fndef, args);
    match fnkind {
        FnKind::Unwind(i) => self.symex_function(i, args, dest, target),
        FnKind::Layout(l) => self.symex_assign_layout(dest, l),
        FnKind::Allocation(k, l) => {
          let object = self.symex_alloc(l);
          let pt = self.make_project(dest);
          let address_of =
            self.ctx.address_of(object.clone(), pt.ty());
          
          self.do_assignment(pt, address_of);
          
          // TODO - do assignment for constant
        },
        FnKind::AsRef => {},
    };
    if matches!(fnkind, FnKind::Unwind(_)) { return; }
    if let Some(t) = target {
      let state = self.cur_frame().cur_state().clone();
      self.cur_frame().add_state(*t, state);
      self.cur_frame().inc_pc();
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
    self.exec_state.push_frame(i, dest.clone(), *target);

    // Set arguements
    let args = self.cur_frame().function().args();
    if !args.is_empty() {
      for arg_local in args.iter() {
        let lhs = self.cur_frame().l0_local(*arg_local);
        let rhs = arg_exprs[*arg_local - 1].clone();
        self.do_assignment(lhs, rhs);
      }
      let state = self.cur_frame().cur_state().clone();
      self.cur_frame().add_state(0, state);
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
    
    let n = self.cur_frame().function().size();
    let mut state = self.cur_frame().cur_state().clone();
    // remove local
    for local in 1..self.cur_frame().function().locals().len() {
      if self.cur_frame().function().local_decl(local).0.is_any_ptr() {
        let l1_count = self.cur_frame().l1_local_count(local);
        for l1_num in 1..l1_count + 1 {
          let pt = self.cur_frame().l1_local(local, l1_num);
          state.remove_pointer(pt);
        }
      }
    }
    self.cur_frame().add_state(n, state);

    self.cur_frame().inc_pc();
  }

}