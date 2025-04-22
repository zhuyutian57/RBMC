use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::*;
use crate::program::program::bigint_to_u64;
use crate::symbol::symbol::*;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_cast(&mut self, kind: CastKind, expr: Expr, ty: Type) -> Expr {
        let target_ty = self.ctx.mk_type(ty);
        match kind {
            CastKind::PointerCoercion(PointerCoercion::Unsize)
            | CastKind::IntToInt
            | CastKind::PtrToPtr => self.ctx.cast(expr, target_ty),
            CastKind::Transmute => {
                if expr.ty().is_nonnull() {
                    let object = self.ctx.object(expr);
                    let i = self.ctx.constant_usize(0);
                    self.ctx.index(object, i, ty)
                } else {
                    todo!()
                }
            }
            _ => todo!("{kind:?} - {expr:?} -> {ty:?}"),
        }
    }
}
