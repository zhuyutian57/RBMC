use num_bigint::BigInt;
use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_cast(&mut self, kind: CastKind, operand: &Operand, ty: Type) -> Expr {
        let expr = self.make_operand(operand).unwrap_predicates();
        match kind {
            CastKind::PointerExposeAddress => self.symex_cast_poniter_expose_address(operand, ty),
            CastKind::PointerCoercion(c) => self.symex_cast_pointer_coercion(c, expr, ty),
            CastKind::IntToInt => self.symex_cast_inttoint(expr, ty),
            CastKind::PtrToPtr => self.symex_cast_ptrtoptr(expr, ty),
            CastKind::Transmute => self.symex_cast_transmute(expr, ty),
            _ => todo!("{kind:?} - {expr:?} -> {ty:?}"),
        }
    }

    fn symex_cast_poniter_expose_address(&mut self, operand: &Operand, ty: Type) -> Expr {
        let pt = self.make_operand(operand);
        self.ctx.cast(pt, self.ctx.mk_type(ty))
    }

    fn symex_cast_pointer_coercion(
        &mut self,
        coercion: PointerCoercion,
        pt: Expr,
        target_ty: Type,
    ) -> Expr {
        let src_ty = pt.ty();
        match coercion {
            PointerCoercion::MutToConstPointer | PointerCoercion::ArrayToPointer => {
                todo!("Support later")
            }
            PointerCoercion::Unsize => {
                if src_ty.pointee_ty().is_array() && target_ty.is_slice_ptr() {
                    let address = pt.clone();
                    let len = src_ty.pointee_ty().array_len().unwrap();
                    let meta = self.ctx.constant_usize(len);
                    self.ctx.pointer(address, Some(meta), target_ty)
                } else {
                    todo!("{src_ty:?} => {target_ty:?}")
                }
            }
            _ => todo!("Unsupport function pointer"),
        }
    }

    fn symex_cast_inttoint(&mut self, expr: Expr, ty: Type) -> Expr {
        // TODO: cast follow the type information
        self.ctx.cast(expr, self.ctx.mk_type(ty))
    }

    fn symex_cast_ptrtoptr(&mut self, pt: Expr, ty: Type) -> Expr {
        if pt.ty().is_slice_ptr() {
            let meta = self.ctx.pointer_meta(pt.clone());
            if ty.is_slice_ptr() {
                self.ctx.pointer(pt.clone(), Some(meta), ty)
            } else {
                self.ctx.pointer(pt.clone(), None, ty)
            }
        } else {
            self.ctx.pointer(pt, None, ty)
        }
    }

    fn symex_cast_transmute(&mut self, expr: Expr, target_ty: Type) -> Expr {
        if expr.ty().is_nonnull() {
            let object = self.ctx.object(expr);
            let i = self.ctx.constant_usize(0);
            self.ctx.index(object, i, target_ty)
        } else if expr.ty().is_integer() && target_ty.is_primitive_ptr() {
            let mut num = expr;
            self.rename(&mut num);
            assert!(num.is_constant() && num.extract_constant().to_integer() == BigInt::ZERO);
            // Create a null pointer
            self.ctx.null(target_ty)
        } else {
            self.ctx.cast(expr, self.ctx.mk_type(target_ty))
        }
    }
}
