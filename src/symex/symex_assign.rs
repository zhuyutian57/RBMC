use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::*;
use crate::program::program::bigint_to_u64;
use crate::symbol::symbol::*;

type BinOp = crate::expr::op::BinOp;
type UnOp = crate::expr::op::UnOp;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_assign(&mut self, place: &Place, rvalue: &Rvalue) {
        // println!("{place:?} - {rvalue:?}");
        // construct lhs expr and rhs expr from MIR
        let lhs = self.make_project(place);
        let rhs = self.make_rvalue(rvalue);
        self.assign(lhs, rhs.clone(), self.ctx._true().into());
    }

    pub(super) fn symex_assign_layout(&mut self, lhs: Expr, ty: Type) {
        // Use l2 symbol to do assignment
        let layout = self.ctx.mk_type(ty);
        self.assign(lhs, layout, self.ctx._true().into());
    }

    pub(super) fn assign(&mut self, lhs: Expr, rhs: Expr, guard: Guard) {
        assert!(lhs.ty().is_layout() || lhs.ty() == rhs.ty());
        self.assign_rec(lhs, rhs.clone(), guard);
        // move semantic
        self.symex_move(rhs);
    }

    fn assign_symbol(&mut self, mut lhs: Expr, mut rhs: Expr, guard: Guard) {
        assert!(lhs.is_symbol());

        if !guard.is_true() {
            rhs = self.ctx.ite(guard.to_expr(), rhs, lhs.clone());
        }

        // Rename to l2 rhs
        self.replace_predicates(&mut rhs);
        self.rename(&mut rhs);
        rhs.simplify();
        // New l2 symbol
        lhs = self.exec_state.new_symbol(&lhs, Level::Level2);

        self.exec_state.assign(lhs.clone(), rhs.clone());

        if rhs.is_type() {
            return;
        }

        // Build VC system
        self.vc_system.borrow_mut().assign(lhs, rhs);
    }

    fn assign_rec(&mut self, lhs: Expr, rhs: Expr, guard: Guard) {
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

            let mut true_guard = guard.clone();
            let mut true_cond = cond.clone();
            self.rename(&mut true_cond);
            true_guard.add(true_cond);
            self.assign_rec(true_value, rhs.clone(), true_guard);

            let mut false_guard = guard.clone();
            let mut false_cond = self.ctx.not(cond.clone());
            self.rename(&mut false_cond);
            false_guard.add(false_cond);
            self.assign_rec(false_value, rhs.clone(), false_guard);

            return;
        }

        if lhs.is_index() {
            let inner_object = lhs.extract_object();
            let mut new_lhs = inner_object.clone();
            let mut index = lhs.extract_index();

            if inner_object.ty().is_slice() {
                let slice = inner_object.extract_inner_expr();
                new_lhs = slice.extract_object();
                index = self.ctx.add(index, slice.extract_slice_start());
            }
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
                let object = self.ctx.object(place);
                let address_of = self.ctx.address_of(object, ty);
                address_of
            }
            Rvalue::Aggregate(k, operands) => self.make_aggregate(k, operands, ty),
            Rvalue::BinaryOp(mir_op, lop, rop) => {
                let op = BinOp::from(mir_op.clone());
                let lhs = self.make_operand(lop);
                let rhs = self.make_operand(rop);
                let expr = match op {
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
            }
            Rvalue::UnaryOp(mir_op, o) => {
                let op = UnOp::from(mir_op.clone());
                let operand = self.make_operand(o);
                let expr = match op {
                    UnOp::Not => self.ctx.not(operand),
                    UnOp::Neg => self.ctx.neg(operand),
                    UnOp::Meta => self.ctx.pointer_meta(operand),
                };
                expr
            }
            Rvalue::Cast(_, operand, t) => {
                // TODO: handle cast kind
                let op = self.make_operand(operand);
                let target_ty = self.ctx.mk_type(Type::from(t.clone()));
                let cast = self.ctx.cast(op, target_ty);
                cast
            }
            Rvalue::Ref(_, _, p) => {
                let place = self.make_project(p);
                let object = self.ctx.object(place);
                let address_of = self.ctx.address_of(object, ty);
                address_of
            }
            Rvalue::Use(operand) => self.make_operand(operand),
            Rvalue::Repeat(operand, tyconst) => {
                let value = self.make_operand(operand);
                let len_expr = self.make_tyconst(tyconst);
                // Carefully for bits
                let bigint = len_expr.extract_constant().to_integer();
                let len = bigint_to_u64(&bigint);
                self.ctx.constant_array(value, Some(len))
            }
            _ => todo!("{rvalue:?}"),
        }
    }

    fn make_aggregate(&mut self, k: &AggregateKind, operands: &Vec<Operand>, ty: Type) -> Expr {
        match k {
            AggregateKind::Array(..) => assert!(ty.is_array()),
            AggregateKind::Adt(..) => assert!(ty.is_struct()),
            AggregateKind::RawPtr(..) => assert!(ty.is_ptr()),
            _ => todo!(),
        };
        let operand_exprs = operands.iter().map(|o| self.make_operand(o)).collect::<Vec<Expr>>();
        self.ctx.aggregate(operand_exprs, ty)
    }
}
