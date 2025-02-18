use std::fmt::Error;

use stable_mir::CrateDef;
use stable_mir::mir::*;
use stable_mir::target::*;
use stable_mir::ty::*;

use crate::expr::context::*;
use crate::expr::constant::*;
use crate::expr::expr::*;
use crate::expr::op::BinOp;
use crate::expr::op::UnOp;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::solvers::solver::Solver;
use crate::symbol::nstring::*;
use crate::program::program::*;
use crate::symbol::symbol::*;
use crate::vc::vc::*;
use crate::config::Config;
use super::exec_state::*;
use super::frame::*;
use super::place_state::*;
use super::projection::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocKind {
  Alloc,
  Box,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FnKind {
  Unwind(FunctionIdx),
  Layout(Type),
  Allocation(AllocKind, Type),
  AsMut(Operand),
  AsRef(Operand),
}

pub struct Symex<'sym> {
  program : &'sym Program,
  ctx: ExprCtx,
  pub(super) exec_state: ExecutionState<'sym>,
  pub(super) vc_system: VCSysPtr,
}

impl<'sym> Symex<'sym> {
  pub fn new(
    program: &'sym Program,
    ctx: ExprCtx,
    vc_system: VCSysPtr) -> Self {
    let mut exec_state = ExecutionState::new(program, ctx.clone());
    exec_state.setup();
    Symex { program, ctx, exec_state, vc_system }
  }

  pub fn can_exec(&self) -> bool { self.exec_state.can_exec() }

  pub fn symex(&mut self) {
    while let Some(pc) = self.top().cur_pc() {
      // Merge states
      if self.exec_state.merge_states(pc) {
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

  fn top(&mut self) -> &mut Frame<'sym> {
    self.exec_state.top_mut()
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

  /// Interface to do projection
  fn make_project(&mut self, place: &Place) -> Expr {    
    Projector::new(self).project(place)
  }

  fn make_mirconst(&mut self, mirconst: &MirConst) -> Expr {
    match mirconst.kind() {
      ConstantKind::Allocated(allocation) => {
        let ty = Type::from(mirconst.ty());
        let fields =
          if ty.is_struct() { ty.struct_def().1 }
          else { vec![(NString::EMPTY, ty)] };
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
          if fields[i].1.is_bool() {
            assert!(raw_bytes.len() == 1);
            value_vec.push(Constant::Bool(raw_bytes[0] == 1));
            continue;
          }
          let (sign, value) =
            read_target_integer(
              raw_bytes.as_slice(),
              fields[i].1.is_signed()
            );
          value_vec.push(Constant::Integer(sign, value));
        }

        if ty.is_struct() {
          Ok(self.ctx.constant_struct(value_vec, ty))
        } else {
          assert!(value_vec.len() == 1);
          if ty.is_bool() {
            Ok(self.ctx.constant_bool(value_vec[0].bool_value()))
          } else if ty.is_integer() {
            let (s, u) = value_vec[0].integer_value();
            Ok(self.ctx.constant_integer(s, u, ty))
          } else {
            Err(Error)
          }
        }
      }
      _ => Err(Error),
    }.expect("Not support")
  }

  fn make_operand(&mut self, operand: &Operand) -> Expr {
    match operand {
      Operand::Copy(p) => {
        // TODO: handle copy semantic?
        self.make_project(p)
      },
      Operand::Move(p) => {
        let place = self.make_project(p);
        self.exec_state.update_place_state(place.clone(), PlaceState::Moved);
        place
      },
      Operand::Constant(op) 
        => self.make_mirconst(&op.const_),
    }
  }

  /// Create l1 formula from Rvalue(MIR)
  fn make_rvalue(&mut self, rvalue: &Rvalue) -> Expr {
    let ty = self.top().function().rvalue_type(rvalue);
    match rvalue {
      Rvalue::AddressOf(m, p) => {
        let place = self.make_project(p);
        let address_of = self.ctx.address_of(place, ty);
        Ok(address_of)
      },
      Rvalue::Aggregate(k, operands) => {
        // println!("{k:?}\n{:?}", operands.len());
        todo!()
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
      Rvalue::Cast(_, operand, t) => {
        // TODO: handle cast kind
        let op = self.make_operand(operand);
        let target_ty = self.ctx.mk_type(Type::from(t.clone()));
        let cast = self.ctx.cast(op, target_ty);
        Ok(cast)
      },
      Rvalue::Ref(_, _, p) => {
        let object = self.make_project(p);
        // TODO: handle borrow kind.
        let address_of = self.ctx.address_of(object, ty);
        Ok(address_of)
      },
      Rvalue::Use(operand)
        => Ok(self.make_operand(operand)),
      _ => Err(Error),
    }.expect(format!("Do not support: {rvalue:?}").as_str())
  }

  fn symex_assign(&mut self, place: &Place, rvalue: &Rvalue) {
    // construct lhs expr and rhs expr from MIR
    let lhs = self.make_project(place);
    let rhs = self.make_rvalue(rvalue);
    self.do_assignment(lhs, rhs);
  }

  fn symex_assign_layout(&mut self, place: &Place, ty: Type) {
    // Use l2 symbol to do assignment
    let l2_var = self.make_project(place);
    let layout = self.ctx.mk_type(ty);
    self.do_assignment(l2_var, layout);
  }

  fn do_assignment(&mut self, lhs: Expr, rhs: Expr) {
    assert!(lhs.ty().is_layout() || lhs.ty() == rhs.ty());
    // TODO: do more jobs
    self.assign_rec(lhs, rhs, self.ctx.constant_bool(true));
  }

  fn assign_symbol(&mut self, mut lhs: Expr, mut rhs: Expr, guard: Expr) {
    assert!(lhs.is_symbol());

    let mut new_guard =
      self.ctx.and(guard, self.exec_state.cur_state().guard());
    self.exec_state.rename(&mut new_guard, Level::Level2);
    new_guard.simplify();

    // Rename to l2 rhs
    self.exec_state.rename(&mut rhs, Level::Level2);
    // New l2 symbol
    lhs = self.exec_state.new_symbol(&lhs, Level::Level2);

    self.exec_state.assign(lhs.clone(), rhs.clone(), new_guard.clone());

    if rhs.is_type() { return; }

    // Build VC system
    self.vc_system.borrow_mut().assign(new_guard, lhs, rhs);
  }

  fn assign_rec(&mut self, lhs: Expr, rhs: Expr, guard: Expr) {
    if lhs.is_symbol() {
      self.assign_symbol(lhs, rhs, guard);
      return;
    }

    if lhs.is_object() {
      let new_lhs = lhs.extract_inner_object();
      self.assign_rec(new_lhs, rhs, guard);
      return;
    }

    if lhs.is_ite() {
      let sub_exprs = lhs.sub_exprs().unwrap();
      let cond = sub_exprs[0].clone();
      let true_value = sub_exprs[1].clone();
      let false_value = sub_exprs[2].clone();
      
      let true_guard = self.ctx.and(guard.clone(), cond.clone());
      self.assign_rec(true_value, rhs.clone(), true_guard);
      
      let false_guard =
        self.ctx.and(
          guard.clone(),
          self.ctx.not(cond.clone())
        );
      self.assign_rec(false_value, rhs.clone(), false_guard);

      return;
    }

    if lhs.is_index_of() {
      let new_lhs = lhs.extract_inner_object();
      let index = lhs.extract_index();
      let new_rhs = self.ctx.store(new_lhs.clone(), index, rhs.clone());
      self.assign_rec(new_lhs, new_rhs, guard);
      return;
    }

    panic!("Do not support assignment:\n{lhs:?} = {rhs:?}");
  }

  fn symex_storagelive(&mut self, local: Local) {
    let var = self.exec_state.new_local(local, Level::Level1);
    if var.ty().is_any_ptr() {
      self.exec_state.cur_state_mut().add_pointer(var);
    }
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

  fn symex_goto(&mut self, target: &BasicBlockIdx) {
    let state = self.top().cur_state().clone();
    self.top().add_state(*target, state);
    self.top().inc_pc();
  }

  fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
    for pc in targets.all_targets() {
      let state = self.top().cur_state().clone();
      // TODO - set path condition
      self.top().add_state(pc, state);
    }
    self.top().inc_pc();
  }

  fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
    let state = self.top().cur_state().clone();
    // TODO: exec drop
    self.top().add_state(*target, state);
    self.top().inc_pc();
  }

  fn make_layout(&mut self, arg: &Operand) -> Type {
    match arg {
      Operand::Move(p) => {
        assert!(p.projection.is_empty());
        let mut ty =
          self.exec_state.current_local(p.local, Level::Level2);
        self.exec_state.rename(&mut ty, Level::Level2);
        assert!(ty.is_type());
        Ok(ty.extract_layout())
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
      Ok(FnKind::Unwind(self.program.function_idx(name)))
    } else if name == NString::from("Layout::new") {
      assert!(fndef.1.0.len() == 1);
      let ty = fndef.1.0[0].ty().unwrap();
      Ok(FnKind::Layout(Type::from(*ty)))
    } else if name == NString::from("Box::<T>::new") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
      Ok(FnKind::Allocation(AllocKind::Box, ty))
    } else if name == NString::from("alloc") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
      Ok(FnKind::Allocation(AllocKind::Alloc, ty))
    } else if name == NString::from("AsMut::as_mut") {
      Ok(FnKind::AsMut(args[0].clone()))
    } else {
      Err(Error)
    }.expect(format!("Do not support {name:?}").as_str())
  }

  fn symex_call(
    &mut self,
    func: &Operand,
    args: &Vec<Operand>,
    dest: &Place,
    target: &Option<BasicBlockIdx>
  ) {
    let ty = self.top().function().operand_type(func);
    let fndef = ty.fn_def();
    let fnkind = self.make_fn_kind(fndef, args);
    match &fnkind {
        FnKind::Unwind(i) => self.symex_function(*i, args, dest, target),
        FnKind::Layout(l) => self.symex_assign_layout(dest, *l),
        FnKind::Allocation(k, t) => {
          let object = self.symex_alloc(*t, *k);
          let pt = self.make_project(dest);
          let address_of = self.ctx.address_of(object.clone(), pt.ty());
          
          self.do_assignment(pt, address_of);
          
          // TODO - do assignment for constant

          let object_state =
            if matches!(k, AllocKind::Box) {
              PlaceState::Initialized
            } else {
              PlaceState::Uninitialized
            };
          self.exec_state.update_place_state(object, object_state);
        },
        FnKind::AsMut(o) => self.symex_as_mut(dest, o),
        FnKind::AsRef(o) => {},
    };
    if matches!(fnkind, FnKind::Unwind(_)) { return; }
    if let Some(t) = target {
      let state = self.top().cur_state().clone();
      self.top().add_state(*t, state);
      self.top().inc_pc();
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
    let args = self.top().function().args();
    if !args.is_empty() {
      for arg_local in args.iter() {
        let lhs = self.exec_state.l0_local(*arg_local);
        let rhs = arg_exprs[*arg_local - 1].clone();
        self.do_assignment(lhs, rhs);
      }
      let state = self.top().cur_state().clone();
      self.top().add_state(0, state);
    }
  }

  fn symex_alloc(&mut self, ty: Type, kind: AllocKind) -> Expr {
    let object = self.exec_state.new_object(ty);
    assert!(object.extract_ownership().is_not());
    if kind == AllocKind::Box {
      let inner_object = object.sub_exprs().unwrap().remove(0);
      return self.ctx.object(inner_object, Ownership::Own);
    }
    object
  }

  fn symex_as_mut(&mut self, place: &Place, operand: &Operand) {
    todo!()
    // let lhs = self.make_project(&place);
    // let target_ty = self.ctx.mk_type(lhs.ty());
    // let o = self.make_operand(operand);
    // let cast = self.ctx.cast(o, target_ty);
    // self.do_assignment(lhs, cast);
  }

  fn symex_return(&mut self) {
    // TODO: set return value and register state
    // to be merged into stack
    
    let n = self.top().function().size();
    let mut state = self.top().cur_state().clone();
    // remove local
    for local in 1..self.top().function().locals().len() {
      let l1_count = self.exec_state.l1_local_count(local);
      for l1_num in 1..l1_count + 1 {
        let l1_local = self.exec_state.l1_local(local, l1_num);
        if l1_local.ty().is_any_ptr() {
          state.remove_pointer(l1_local.clone());
        }
      }
    }
    state.remove_stack_places();
    self.top().add_state(n, state);

    self.top().inc_pc();
  }

}