
use std::fmt::Error;

use stable_mir::mir::AggregateKind;
use stable_mir::mir::Operand;
use stable_mir::mir::Rvalue;
use stable_mir::mir::Place;
use stable_mir::target::*;
use stable_mir::ty::*;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::constant::*;
use crate::expr::op::*;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::program::program::*;
use crate::symbol::symbol::*;
use crate::symbol::nstring::*;
use super::frame::Pc;
use super::place_state::*;
use super::projection::*;
use super::state::State;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn merge_states(&mut self, pc: Pc) -> bool {
    let state_vec = self.top().states_from(pc);

    // We have put all states that reach current pc in the
    // queue. Thus, we first construct an empty state.
    // That is, make `gurad` of current state be `false`.
    self.top().cur_state.guard = self.ctx._false();

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

    let mut nrenaming =
      nstate.renaming.as_ref().unwrap().borrow_mut();

    for var in nrenaming.variables() {
      let l1_ident =
        nrenaming.current_l1_symbol(var).l1_name();
      
      let cur_l2_num =
        self.exec_state.renaming.borrow().l2_count(l1_ident);
      let n_l2_num =
        nrenaming.l2_count(l1_ident);

      if cur_l2_num == n_l2_num  || n_l2_num == 0 { continue; }
      
      let mut cur_rhs = self.exec_state.ns.lookup_symbol(var);
      let mut new_rhs = self.exec_state.ns.lookup_symbol(var);

      // Get l1 number
      nrenaming.l1_rename(&mut cur_rhs);
      nrenaming.l1_rename(&mut new_rhs);

      // Current assignment
      self.exec_state.renaming.borrow_mut().l2_rename(&mut cur_rhs);
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
        
      let mut lhs= self.exec_state.ns.lookup_symbol(var);
      lhs = self.exec_state.new_symbol(&lhs, Level::Level2);
      
      self.exec_state.assign(lhs.clone(), rhs.clone());

      self.vc_system.borrow_mut().assign(lhs, rhs);
    }
  }

  fn memory_leak_check(&self) {
    for object in &self.exec_state.objects {
      if object.extract_ownership().is_own() { continue; }

      let msg = 
        NString::from(format!("memory leak: {object:?} is not dealloced"));
      let alloac_array =
        self.exec_state.ns.lookup_symbol(NString::ALLOC_SYM);
      let address_of =
        self.ctx.address_of(object.clone(), object.extract_address_type());
      let is_leak = 
        self.ctx.index(
          alloac_array,
          address_of,
          Type::bool_type()
        );
      self.claim(msg, is_leak);
    }
  }

  pub(super) fn make_project(&mut self, place: &Place) -> Expr {
    Projection::new(self).project(place)
  }

  pub(super) fn make_deref(
    &mut self,
    pt: Expr,
    mode: Mode,
    guard: Expr
  ) -> Option<Expr> {
    Projection::new(self).project_deref(pt, mode, guard)
  }

  pub(super) fn make_mirconst(&mut self, mirconst: &MirConst) -> Expr {
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

  pub(super) fn make_operand(&mut self, operand: &Operand) -> Expr {
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
  pub(super) fn make_rvalue(&mut self, rvalue: &Rvalue) -> Expr {
    let ty = self.top().function().rvalue_type(rvalue);
    match rvalue {
      Rvalue::AddressOf(m, p) => {
        let place = self.make_project(p);
        let address_of = self.ctx.address_of(place, ty);
        address_of
      },
      Rvalue::Aggregate(k, operands) => {
        self.make_aggregate(k, operands, ty)
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
        expr
      },
      Rvalue::UnaryOp(mir_op, o) => {
        let op = UnOp::from(mir_op.clone());
        let operand = self.make_operand(o);
        let expr =
          match op {
            UnOp::Not => self.ctx.not(operand),
            UnOp::Neg => self.ctx.neg(operand),
          };
        expr
      },
      Rvalue::Cast(_, operand, t) => {
        // TODO: handle cast kind
        let op = self.make_operand(operand);
        let target_ty = self.ctx.mk_type(Type::from(t.clone()));
        let cast = self.ctx.cast(op, target_ty);
        cast
      },
      Rvalue::Ref(_, _, p) => {
        let object = self.make_project(p);
        // TODO: handle borrow kind.
        let address_of = self.ctx.address_of(object, ty);
        address_of
      },
      Rvalue::Use(operand) => self.make_operand(operand),
      _ => todo!(),
    }
  }

  pub(super) fn make_layout(&mut self, arg: &Operand) -> Type {
    match arg {
      Operand::Move(p) => {
        assert!(p.projection.is_empty());
        let mut ty =
          self.exec_state.current_local(p.local, Level::Level2);
        self.rename(&mut ty);
        assert!(ty.is_type());
        Ok(ty.extract_layout())
      },
      Operand::Constant(c) => {
        Ok(Type::from(c.ty()))
      }
      _ => Err(Error),
    }.expect("Do no exits")
  }

  fn make_aggregate(
    &mut self,
    k: &AggregateKind,
    operands: &Vec<Operand>,
    ty: Type
  ) -> Expr {
    match k {
      AggregateKind::Array(..) => assert!(ty.is_array()),
      AggregateKind::Adt(..) => assert!(ty.is_struct()),
      AggregateKind::RawPtr(..) => assert!(ty.is_ptr()),
      _ => todo!(),
    };
    let operand_exprs =
      operands
        .iter()
        .map(|o| self.make_operand(o))
        .collect::<Vec<Expr>>();
    self.ctx.aggregate(operand_exprs, ty)
  }

  /// Interface for `l2` reaming.
  pub(super) fn rename(&self, expr: &mut Expr) {
    self.exec_state.rename(expr, Level::Level2);
  }

  pub(super) fn replace_predicates(&self, expr: &mut Expr) {
    match expr.sub_exprs() {
      Some(mut sub_exprs) => {
        let mut has_changed = false;
        for sub_expr in sub_exprs.iter_mut() {
          if sub_expr.has_predicates() {
            has_changed |= true;
            self.replace_predicates(sub_expr);
          }
        }
        if has_changed { expr.replace_sub_exprs(sub_exprs); }
      }
      None => {},
    }

    if expr.is_invalid() {
      let object = expr.extract_object();
      let ptr_indent =
        self
          .ctx
          .pointer_ident(
              self.ctx.address_of(
                object.clone(),
                object.extract_address_type()
              )
          );
      let alloc_array =
        self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
      let not_alloced =
        self.ctx.not(
          self.ctx.index(
            alloc_array,
            ptr_indent,
            Type::bool_type()
          )
        );
      *expr = not_alloced;
      return;
    }
  }

  pub(super) fn claim(&self, msg: NString, mut expr: Expr) {
    self.replace_predicates(&mut expr);
    self.rename(&mut expr);
    expr.simplify();
    let mut guard = self.exec_state.cur_state().guard();
    let cond = self.ctx.implies(guard, expr);
    self.vc_system.borrow_mut().assert(msg, cond);
  }
}