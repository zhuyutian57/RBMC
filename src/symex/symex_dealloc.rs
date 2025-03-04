
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::NString;
use super::symex::*;
use super::projection::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_dealloc(&mut self, pt: Expr, ty: Type) {
    assert!(pt.ty().is_ptr());
    // Generate assertions
    self.make_deref(pt.clone(), Mode::Dealloc, self.ctx._true());

    self.top_mut().cur_state.dealloc_objects(pt.clone());
    self.top_mut().cur_state.remove_pointer(pt.clone());

    let pointer_ident = self.ctx.pointer_ident(pt);
    let alloc_array =
      self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
    let index =
      self.ctx.index(alloc_array, pointer_ident, Type::bool_type());
    self.assign(index, self.ctx._false(), self.ctx._true());
  }
}