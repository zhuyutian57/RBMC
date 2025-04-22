use stable_mir::mir::*;
use stable_mir::ty::IndexedVal;

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
        // construct lhs expr and rhs expr from MIR
        let lhs = self.make_project(place);
        let rhs = self.make_rvalue(rvalue);
        self.assign(lhs, rhs.clone(), self.ctx._true().into());
    }

    pub(super) fn assign(&mut self, lhs: Expr, rhs: Expr, guard: Guard) {
        assert!(lhs.ty().is_layout() || lhs.ty() == rhs.ty());
        self.assign_rec(lhs, rhs.clone(), guard);
    }

    fn assign_symbol(&mut self, mut lhs: Expr, mut rhs: Expr, guard: Guard) {
        assert!(lhs.is_symbol() && !lhs.extract_symbol().is_level2());

        if !guard.is_true() {
            rhs = self.ctx.ite(guard.to_expr(), rhs, lhs.clone());
        }

        // Rename to l1 rhs
        self.exec_state.rename(&mut lhs, Level::Level1);

        // Rename to l2 rhs
        self.replace_predicates(&mut rhs);
        self.rename(&mut rhs);
        rhs.simplify();

        // Assignment for symex
        self.exec_state.assignment(lhs.clone(), rhs.clone());

        // New l2 symbol
        lhs = self.exec_state.new_symbol(&lhs, Level::Level2);

        if rhs.is_type() {
            return;
        }

        // Build VC system
        self.vc_system.borrow_mut().assign(lhs, rhs, self.exec_state.span);
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

        if lhs.is_as_variant() {
            let new_lhs = lhs.extract_enum();
            self.assign_rec(new_lhs, rhs, guard);
            return;
        }

        panic!("Do not support assignment:\n{lhs:?} = {rhs:?}");
    }

    fn make_rvalue(&mut self, rvalue: &Rvalue) -> Expr {
        let ty = self.top_mut().function.rvalue_type(rvalue);
        match rvalue {
            Rvalue::AddressOf(_, p) => {
                let place = self.make_project(p);
                let object = if place.is_object() {
                    place
                } else {
                    self.ctx.object(place)
                };
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
                    BinOp::Offset => self.ctx.offset(lhs, rhs),
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
            Rvalue::Cast(k, op, ty) => {
                let expr = self.make_operand(op);
                self.symex_cast(*k, expr, Type::from(ty))
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
            Rvalue::Discriminant(p) => {
                let place = self.make_project(p);
                assert!(place.ty().is_enum());
                let def = place.ty().enum_def();
                let mut discr = self.ctx.constant_isize(0);
                for i in 1..def.1.len() {
                    let idx = self.ctx.constant_isize(i as isize);
                    let cond = self.ctx.match_variant(place.clone(), idx.clone());
                    discr = self.ctx.ite(cond, idx, discr);
                }
                discr
            }
            _ => todo!("{rvalue:?}"),
        }
    }

    fn make_aggregate(&mut self, k: &AggregateKind, operands: &Vec<Operand>, ty: Type) -> Expr {
        let args = operands
            .iter()
            .map(|o| self.make_operand(o))
            .collect::<Vec<Expr>>();
        match k {
            AggregateKind::Array(..) => {
                assert!(ty.is_array());
                self.ctx.aggregate(args, ty)
            }
            AggregateKind::Adt(def, i, ..) => {
                assert!(ty.is_struct() || ty.is_tuple() || ty.is_enum());
                if ty.is_struct() || ty.is_tuple() {
                    self.ctx.aggregate(args, ty)
                } else {
                    let idx = self.ctx.constant_usize(i.to_index());
                    if args.len() == 0 {
                        self.ctx.constant_adt(vec![idx.extract_constant()], ty)
                    } else {
                        let tuple_ty = ty.enum_variant_data_type(i.to_index());
                        let data = self.ctx.aggregate(args, tuple_ty);
                        self.ctx.variant(idx, data, ty)
                    }
                }
            }
            AggregateKind::RawPtr(t, m) => {
                assert!(ty.pointee_ty() == Type::from(t));
                let pt = args[0].clone();
                let base = self.ctx.pointer_base(pt.clone());
                let offset = self.ctx.pointer_offset(pt);
                let meta = args[1].clone();
                self.ctx.pointer(base, offset, Some(meta), ty)
            }
            _ => todo!("{k:?}"),
        }
    }
}
