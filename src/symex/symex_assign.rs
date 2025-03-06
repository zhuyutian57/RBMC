
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::symbol::*;
use crate::symex::place_state::NPlace;
use crate::symex::place_state::PlaceState;
use super::symex::*;

type BinOp = crate::expr::op::BinOp;
type UnOp = crate::expr::op::UnOp;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_assign(&mut self, place: &Place, rvalue: &Rvalue) {
    // construct lhs expr and rhs expr from MIR
    let lhs = self.make_project(place);
    let rhs = self.make_rvalue(rvalue);
    self.assign(lhs, rhs.clone(), self.ctx._true());
    // move semantic
    self.symex_move(rhs);
  }

  pub(super) fn symex_assign_layout(&mut self, place: &Place, ty: Type) {
    // Use l2 symbol to do assignment
    let l2_var = self.make_project(place);
    let layout = self.ctx.mk_type(ty);
    self.assign(l2_var, layout, self.ctx._true());
  }

  pub(super) fn assign(&mut self, lhs: Expr, mut rhs: Expr, guard: Expr) {
    assert!(lhs.ty().is_layout() || lhs.ty() == rhs.ty());
    self.replace_predicates(&mut rhs);
    self.assign_rec(lhs, rhs, guard);
  }

  fn assign_symbol(&mut self, mut lhs: Expr, mut rhs: Expr, guard: Expr) {
    assert!(lhs.is_symbol());
    
    if !guard.is_true() {
      rhs = self.ctx.ite(guard, rhs, lhs.clone());
    }

    // Rename to l2 rhs
    self.rename(&mut rhs);
    // New l2 symbol
    lhs = self.exec_state.new_symbol(&lhs, Level::Level2);

    // update new lhs place state
    if self.is_stack_symbol(lhs.clone()) {
      let nplace = NPlace(lhs.extract_symbol().l1_name());
      self.top_mut().cur_state.update_place_state(nplace, PlaceState::Own);
    }

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

  fn make_rvalue(&mut self, rvalue: &Rvalue) -> Expr {
    let ty = self.top_mut().function().rvalue_type(rvalue);
    match rvalue {
      Rvalue::AddressOf(_, p) => {
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
}