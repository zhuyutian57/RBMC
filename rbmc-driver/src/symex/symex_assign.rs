use stable_mir::mir;
use stable_mir::mir::*;
use stable_mir::ty::IndexedVal;
use stable_mir::ty::TyConst;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::*;
use crate::program::program::bigint_to_u64;
use crate::symbol::nstring::NString;
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
        
        if lhs.ty().is_zero_sized_type() || rhs.is_type() {
            return;
        }
        
        // Build VC system
        self.vc_system.borrow_mut().assign(lhs, rhs, self.exec_state.cur_span());
    }

    fn assign_rec(&mut self, lhs: Expr, rhs: Expr, guard: Guard) {
        if lhs.is_invalid_object() { return; }

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
            self.assign_ite(lhs, rhs, guard);
            return;
        }

        if lhs.is_index() {
            let inner_object = lhs.extract_object();
            let mut new_lhs = inner_object.clone();
            let mut index = lhs.extract_index();

            if inner_object.extract_inner_expr().is_slice() {
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

    fn assign_ite(&mut self, lhs: Expr, rhs: Expr, guard: Guard) {
        let sub_exprs = lhs.sub_exprs();
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
    }

    fn make_rvalue(&mut self, rvalue: &Rvalue) -> Expr {
        let ty = self.top_mut().function.rvalue_type(rvalue);
        match rvalue {
            Rvalue::AddressOf(_, place) => self.make_address_of(place, ty),
            Rvalue::Aggregate(k, operands) => self.make_aggregate(k, operands, ty),
            Rvalue::BinaryOp(bop, lop, rop) => self.make_binary(*bop, lop, rop),
            Rvalue::UnaryOp(uop, operand) => self.make_unary(*uop, operand),
            Rvalue::Cast(k, operand, ty) => self.symex_cast(*k, operand, Type::from(ty)),
            Rvalue::Ref(_, _, place) => self.make_address_of(place, ty),
            Rvalue::NullaryOp(nop, t) => self.make_nullary(nop.clone(), t.into()),
            Rvalue::Use(operand) => self.make_operand(operand),
            Rvalue::Repeat(operand, tyconst) => self.make_repeat(operand, tyconst),
            Rvalue::Discriminant(place) => self.make_discriminant(place),
            _ => todo!("{rvalue:?}"),
        }
    }

    fn make_address_of(&mut self, place: &Place, ty: Type) -> Expr {
        let mut object = self.make_project(place);
        if ty.is_slice_ptr() {
            assert!(object.is_slice());
            let root_object = object.extract_object();
            let base = self.ctx.address_of(root_object.clone(), root_object.extract_address_type());
            let start = object.extract_slice_start();
            let address = self.ctx.offset(base, start);
            let meta = object.extract_slice_len();
            self.ctx.pointer(address, Some(meta), ty)
        } else {
            if !object.is_object() {
                object = self.ctx.object(object);
            }
            self.ctx.address_of(object, ty)
        }
    }

    fn make_aggregate(&mut self, k: &AggregateKind, operands: &Vec<Operand>, ty: Type) -> Expr {
        let args = operands.iter().map(|o| self.make_operand(o)).collect::<Vec<Expr>>();
        match k {
            AggregateKind::Array(..) => {
                assert!(ty.is_array());
                self.ctx.aggregate(args, ty)
            }
            AggregateKind::Tuple => {
                assert!(ty.is_tuple());
                self.ctx.aggregate(args, ty)
            }
            AggregateKind::Adt(_, i, ..) => {
                assert!(ty.is_struct() || ty.is_enum());
                if ty.is_struct() {
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
            AggregateKind::RawPtr(t, _) => {
                assert!(ty.pointee_ty() == Type::from(t));
                let address = args[0].clone();
                let meta = args[1].clone();
                self.ctx.pointer(address, Some(meta), ty)
            }
            _ => todo!("{k:?}"),
        }
    }

    fn make_binary(&mut self, bop: mir::BinOp, lop: &Operand, rop: &Operand) -> Expr {
        let op = BinOp::from(bop);
        let lhs = self.make_operand(lop);
        let rhs = self.make_operand(rop);
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
            BinOp::Offset => self.ctx.offset(lhs, rhs),
        }
    }

    fn make_unary(&mut self, uop: mir::UnOp, operand: &Operand) -> Expr {
        let op = UnOp::from(uop);
        let operand = self.make_operand(operand);
        match op {
            UnOp::Not => self.ctx.not(operand),
            UnOp::Neg => self.ctx.neg(operand),
            UnOp::Meta => self.ctx.pointer_meta(operand),
        }
    }

    fn make_nullary(&mut self, nop: NullOp, ty: Type) -> Expr {
        match nop {
            NullOp::UbChecks | NullOp::ContractChecks => self.ctx._false(),
            _ => todo!(),
        }
    }

    fn make_repeat(&mut self, operand: &Operand, tyconst: &TyConst) -> Expr {
        let value = self.make_operand(operand);
        let len_expr = self.make_tyconst(tyconst);
        // Carefully for bits
        let bigint = len_expr.extract_constant().to_integer();
        let len = bigint_to_u64(&bigint);
        self.ctx.constant_array(value, Some(len))
    }

    fn make_discriminant(&mut self, place: &Place) -> Expr {
        let expr = self.make_project(place);
        assert!(expr.ty().is_enum());
        let def = expr.ty().enum_def();
        let mut discr = self.ctx.constant_isize(0);
        for i in 1..def.1.len() {
            let idx = self.ctx.constant_isize(i as isize);
            let cond = self.ctx.match_variant(expr.clone(), idx.clone());
            discr = self.ctx.ite(cond, idx, discr);
        }
        discr
    }
}
