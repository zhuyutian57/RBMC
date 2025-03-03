
use crate::expr::expr::*;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_alloc(&mut self, ty: Type, kind: AllocKind) -> Expr {
    let mut object = self.exec_state.new_object(ty);
    assert!(object.extract_ownership().is_not());
    if kind == AllocKind::Box {
      let inner_object = object.sub_exprs().unwrap().remove(0);
      object = self.ctx.object(inner_object, Ownership::Own);
    }
    self.track_new_object(object.clone());
    object
  }

  pub(super) fn track_new_object(&mut self, object: Expr) {
    assert!(object.is_object());
    let ctx = object.ctx.clone();

    // alloc[&object] = true
    let alloc_array =
      self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
    let pt_indent =
      ctx.pointer_ident(
        ctx.address_of(
          object.clone(),
          object.extract_address_type()
        )
      );
    let store =
      ctx.store(alloc_array.clone(), pt_indent, ctx._true());
    self.assign(alloc_array, store, self.ctx._true());
  }
}