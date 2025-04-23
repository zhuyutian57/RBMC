use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::*;
use crate::program::program::bigint_to_u64;
use crate::symbol::symbol::*;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_cast(&mut self, kind: CastKind, operand: &Operand, ty: Type) -> Expr {
        let expr = self.make_operand(operand);
        match kind {
            CastKind::PointerCoercion(c)
                => self.symex_cast_pointer_coercion(c, expr, ty),
            CastKind::IntToInt | CastKind::PtrToPtr
                => self.ctx.cast(expr, self.ctx.mk_type(ty)),
            CastKind::Transmute 
                => self.symex_cast_transmute(expr, ty),
            _ => todo!("{kind:?} - {expr:?} -> {ty:?}"),
        }
    }

    fn symex_cast_pointer_coercion(
        &mut self,
        coercion: PointerCoercion,
        pt: Expr,
        target_ty: Type
    ) -> Expr {
        let src_ty = pt.ty();
        match coercion {
            PointerCoercion::MutToConstPointer
            | PointerCoercion::ArrayToPointer => todo!("Support later"),
            PointerCoercion::Unsize => {
                if src_ty.pointee_ty().is_array() && target_ty.is_slice_ptr() {
                    let base = pt.clone();
                    let offset = self.ctx.constant_usize(0);
                    let len = src_ty.pointee_ty().array_len().unwrap();
                    let meta = self.ctx.constant_usize(len);
                    self.ctx.pointer(base, offset, meta, target_ty)
                } else {
                    todo!("{src_ty:?} => {target_ty:?}")
                }
            },
            _ => todo!("Unsupport function pointer"),
        }
    }

    fn symex_cast_transmute(&mut self, expr: Expr, target_ty: Type) -> Expr {
        if expr.ty().is_nonnull() {
            let object = self.ctx.object(expr);
            let i = self.ctx.constant_usize(0);
            self.ctx.index(object, i, target_ty)
        } else {
            todo!()
        }
    }

}