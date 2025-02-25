
use std::collections::HashMap;
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
use crate::config::config::Config;
use super::exec_state::*;
use super::frame::*;
use super::place_state::*;
use super::projection::*;
use super::state::State;

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

    let mut symex =
      Symex { program, ctx: ctx.clone(), exec_state, vc_system };
    let alloc_sym = symex.exec_state.ns.lookup(NString::ALLOC_SYM);
    let mut alloc_array = ctx.object(alloc_sym, Ownership::Own);
    let mut const_array =
      ctx.constant_array(Constant::Bool(false), Type::bool_type());
    symex.assign_rec(alloc_array, const_array, ctx.constant_bool(true));
    symex
  }

  pub fn can_exec(&self) -> bool { self.exec_state.can_exec() }

  fn top(&mut self) -> &mut Frame<'sym> {
    self.exec_state.top_mut()
  }

  pub fn symex(&mut self) {
    while let Some(pc) = self.top().cur_pc() {
      // Merge states
      if self.merge_states(pc) {
        // println!(
        //   "Enter {:?} - bb{pc}\n{:?}",
        //   self.top().function().name(),
        //   self.top().cur_state()
        // );
        let bb = self.top().function().basicblock(pc);
        self.symex_basicblock(bb);
      } else {
        self.top().inc_pc();
      }
    }
    self.exec_state.pop_frame();
  }

  fn merge_states(&mut self, pc: Pc) -> bool {
    let state_vec = self.top().states_from(pc);

    // We have put all states that reach current pc in the
    // queue. Thus, we first construct an empty state.
    // That is, make `gurad` of current state be `false`.
    self.top().cur_state.guard = self.ctx.constant_bool(false);

    if let Some(states) = state_vec {
      for mut state in states {
        if state.guard.is_false() { continue; }

        // SSA assigment
        self.phi_function(&mut state);

        self.top().cur_state.merge(&state);
      }
    }

    !self.top().cur_state.guard.is_false()
  }

  fn phi_function(&mut self, nstate: &mut State) {
    if let None = nstate.renaming { return; }

    let mut new_guard =
      self.ctx.and(
        nstate.guard(),
        self.ctx.not(self.exec_state.cur_state().guard())
      );
    new_guard.simplify();

    let nrenaming = nstate.renaming.as_deref_mut().unwrap();

    for var in nrenaming.variables() {
      let l1_ident =
        nrenaming.current_l1_symbol(var).l1_name();
      
      let cur_l2_num = self.exec_state.renaming.l2_count(l1_ident);
      let n_l2_num = nrenaming.l2_count(l1_ident);

      if cur_l2_num == n_l2_num  || n_l2_num == 0 { continue; }
      
      let mut cur_rhs = self.exec_state.ns.lookup(var);
      let mut new_rhs = self.exec_state.ns.lookup(var);

      // Get l1 number
      nrenaming.l1_rename(&mut cur_rhs);
      nrenaming.l1_rename(&mut new_rhs);

      // Current assignment
      self.exec_state.renaming.l2_rename(&mut cur_rhs);
      // Other assignment
      nrenaming.l2_rename(&mut new_rhs);

      let rhs = 
        if self.exec_state.cur_state().guard.is_false() {
          new_rhs
        } else {
          self.ctx.ite(
            new_guard.clone(),
            new_rhs,
            cur_rhs
          )
        };
        
      let mut lhs= self.exec_state.ns.lookup(var);
      lhs = self.exec_state.new_symbol(&lhs, Level::Level2);
      
      self.exec_state.assign(lhs.clone(), rhs.clone());

      self.vc_system.borrow_mut().assign(lhs, rhs);
    }
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
          let value =
            read_target_integer(
              raw_bytes.as_slice(),
              fields[i].1.is_signed()
            );
          value_vec.push(Constant::Integer(value));
        }

        if ty.is_struct() {
          let mut struct_fields = Vec::new();
          for i in 0..fields.len() {
            struct_fields.push((value_vec[i].clone(), fields[i].1.clone()));
          }
          Ok(self.ctx.constant_struct(struct_fields, ty))
        } else {
          assert!(value_vec.len() == 1);
          if ty.is_bool() {
            Ok(self.ctx.constant_bool(value_vec[0].to_bool()))
          } else if ty.is_integer() {
            let i = value_vec[0].to_integer();
            Ok(self.ctx.constant_integer(i, ty))
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
            BinOp::Implies => self.ctx.implies(lhs, rhs),
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
    self.assign(lhs, rhs);
  }

  fn symex_assign_layout(&mut self, place: &Place, ty: Type) {
    // Use l2 symbol to do assignment
    let l2_var = self.make_project(place);
    let layout = self.ctx.mk_type(ty);
    self.assign(l2_var, layout);
  }

  fn assign(&mut self, lhs: Expr, rhs: Expr) {
    assert!(lhs.ty().is_layout() || lhs.ty() == rhs.ty());
    // TODO: do more jobs
    self.assign_rec(lhs, rhs, self.ctx.constant_bool(true));
  }

  fn assign_symbol(&mut self, mut lhs: Expr, mut rhs: Expr, guard: Expr) {
    assert!(lhs.is_symbol());
    
    if !guard.is_true() {
      rhs = self.ctx.ite(guard, rhs, lhs.clone());
    }

    // Rename to l2 rhs
    self.exec_state.rename(&mut rhs, Level::Level2);
    // New l2 symbol
    lhs = self.exec_state.new_symbol(&lhs, Level::Level2);

    self.exec_state.assign(lhs.clone(), rhs.clone());

    if rhs.is_type() { return; }

    // Build VC system
    self.vc_system.borrow_mut().assign(lhs, rhs);
  }

  fn assign_rec(&mut self, lhs: Expr, rhs: Expr, guard: Expr) {
    if lhs.is_symbol() {
      self.assign_symbol(lhs, rhs, guard);
      return;
    }

    if lhs.is_object() {
      let new_lhs = lhs.extract_inner_expr();
      self.assign_rec(new_lhs, rhs, guard);
      return;
    }

    if lhs.is_ite() {
      let sub_exprs = lhs.sub_exprs().unwrap();
      let cond = sub_exprs[0].clone();
      let true_value = sub_exprs[1].clone();
      let false_value = sub_exprs[2].clone();
      
      let mut true_guard = self.ctx.and(guard.clone(), cond.clone());
      true_guard.simplify();
      self.assign_rec(true_value, rhs.clone(), true_guard);
      
      let mut false_guard =
        self.ctx.and(
          guard.clone(),
          self.ctx.not(cond.clone())
        );
      false_guard.simplify();
      self.assign_rec(false_value, rhs.clone(), false_guard);

      return;
    }

    if lhs.is_index() {
      let new_lhs = lhs.extract_object();
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

  fn register_state(&mut self, pc: Pc, mut state: State) {
    state.renaming = Some(Box::new(self.exec_state.renaming.clone()));
    self.top().add_state(pc, state);
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
    self.register_state(*target, state);
    self.top().inc_pc();
  }

  fn symex_switchint(&mut self, discr: &Operand, targets: &SwitchTargets) {
    let discr_expr = self.make_operand(discr);
    if discr_expr.ty().is_bool() {
      let mut true_state = self.top().cur_state().clone();
      true_state.guard =
        self.ctx.and(true_state.guard(), discr_expr.clone());
      self.exec_state.rename(&mut true_state.guard, Level::Level2);
      let true_branch = targets.all_targets()[0];
      self.register_state(true_branch, true_state);

      let mut false_state = self.top().cur_state().clone();
      false_state.guard =
        self.ctx.and(
          false_state.guard.clone(),
          self.ctx.not(discr_expr.clone())
        );
      self.exec_state.rename(&mut false_state.guard, Level::Level2);
      let false_branch = targets.all_targets()[1];
      self.register_state(false_branch, false_state);
    } else if discr_expr.ty().is_integer() {
      let mut state = self.top().cur_state().clone();
      let state_guard = state.guard();
      let mut otherwise_guard = state.guard();
      // branches
      for (i, bb) in targets.branches() {
        let branch_guard =
          self.ctx.eq(
            discr_expr.clone(),
            self.ctx.constant_integer(BigInt(false, i), discr_expr.ty())
          );
        state.guard =
          self.ctx.and(state_guard.clone(), branch_guard.clone());
        self.exec_state.rename(&mut state.guard, Level::Level2);
        self.register_state(bb, state.clone());
        otherwise_guard = 
          self.ctx.and(
            otherwise_guard,
            self.ctx.not(branch_guard)
          );
      }
      // otherwise
      state.guard = otherwise_guard;
      self.register_state(targets.otherwise(), state);
    } else {
      panic!("Not implement {discr:?}");
    }

    self.top().inc_pc();
  }

  fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
    let state = self.top().cur_state().clone();

    // Drop recursively
    let object = self.make_project(place);
    self.symex_drop_rec(object, self.ctx.constant_bool(true));

    self.register_state(*target, state);
    self.top().inc_pc();
  }

  fn symex_drop_rec(&mut self, expr: Expr, guard: Expr) {
    if expr.is_object() {
      if expr.ty().is_box() {
        // TODO: do dereference and add assertion

        let pointer_ident = self.ctx.pointer_ident(expr.extract_inner_expr());      
        let alloc_array_sym = self.exec_state.ns.lookup(NString::ALLOC_SYM);
        let alloc_array = self.ctx.object(alloc_array_sym, Ownership::Own);
        let index =
          self.ctx.index(alloc_array, pointer_ident, Type::bool_type());
        self.assign_rec(index, self.ctx.constant_bool(false), guard.clone());
      } else {
        panic!("drop {:?} should be implemented", expr.ty());
      }
      return;
    }

    if expr.is_ite() {
      let cond = expr.extract_cond();
      let true_value = expr.extract_true_value();
      let false_value = expr.extract_false_value();

      let true_guard =
        self.ctx.and(guard.clone(), cond.clone());
      let false_guard = 
        self.ctx.and(guard.clone(), self.ctx.not(cond));

      self.symex_drop_rec(true_value, true_guard);
      self.symex_drop_rec(false_value, false_guard);
      return;
    }

    panic!("Not implement drop {:?}", expr.ty());
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
          
          self.assign(pt, address_of);
          
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
      self.register_state(*t, state);
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
    self
      .exec_state
      .push_frame(i, Some(dest.clone()), *target);

    // Set arguements
    let args = self.top().function().args();
    if !args.is_empty() {
      for arg_local in args.iter() {
        let lhs = self.exec_state.l0_local(*arg_local);
        let rhs = arg_exprs[*arg_local - 1].clone();
        self.assign(lhs, rhs);
      }
      let state = self.top().cur_state().clone();
      self.register_state(0, state);
    }
  }

  fn symex_alloc(&mut self, ty: Type, kind: AllocKind) -> Expr {
    let mut object = self.exec_state.new_object(ty);
    assert!(object.extract_ownership().is_not());
    if kind == AllocKind::Box {
      let inner_object = object.sub_exprs().unwrap().remove(0);
      object = self.ctx.object(inner_object, Ownership::Own);
    }
    self.track_new_object(object.clone());
    object
  }

  fn track_new_object(&mut self, object: Expr) {
    assert!(object.is_object());
    let ctx = object.ctx.clone();

    // alloc[&object] = true
    let alloc_array =
      self.exec_state.ns.lookup(NString::ALLOC_SYM);
    let pt_indent =
      ctx.pointer_ident(
        ctx.address_of(
          object.clone(),
          object.extract_address_type()
        )
      );
    let store =
      ctx.store(
        alloc_array.clone(),
        pt_indent,
        ctx.constant_bool(true)
      );
    self.assign(alloc_array, store);
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
    // TODO: set return value
    
    let n = self.top().function().size();
    let mut state = self.top().cur_state().clone();
    // TODO: remove local in renaming
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
    self.register_state(n, state);

    self.top().inc_pc();
  }

  /// The common interface for generating assertions
  pub fn claim(&mut self, msg: NString, expr: Expr) {
    let mut guard = self.exec_state.cur_state().guard();
    let cond = self.ctx.implies(guard, expr);
    self
      .vc_system
      .borrow_mut()
      .assert(msg, cond);
  }
}