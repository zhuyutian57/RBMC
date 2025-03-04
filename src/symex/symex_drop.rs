
use stable_mir::mir::*;
use stable_mir::ty::UintTy;

use crate::expr::constant::BigInt;
use crate::expr::expr::*;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::symex::projection::Mode;
use crate::NString;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
    // Drop recursively
    let object = self.make_project(place);
    self.symex_drop_rec(object, self.ctx._true());

    let state = self.top().cur_state().clone();
    self.register_state(*target, state);
    self.top().inc_pc();
  }

  fn symex_drop_rec(&mut self, expr: Expr, guard: Expr) {
    if expr.is_object() {
      if expr.ty().is_box() {
        self.drop_box(expr.clone(), guard.clone());
      } else if expr.ty().is_struct() {
        self.drop_struct(expr.clone(), guard.clone());
      } else {
        panic!("May be just return");
      }
      return;
    }

    if expr.is_ite() {
      let cond = expr.extract_cond();
      let true_value = expr.extract_true_value();
      let false_value = expr.extract_false_value();

      let true_guard =
        self.ctx.and(guard.clone(), cond.clone());
      let false_guard = 
        self.ctx.and(guard.clone(), self.ctx.not(cond));

      self.symex_drop_rec(true_value, true_guard);
      self.symex_drop_rec(false_value, false_guard);
      return;
    }

    panic!("Not implement drop {:?}", expr.ty());
  }

  /// Drop a box will free the memory it points to
  fn drop_box(&mut self, _box: Expr, guard: Expr) {
    // Check whethe the box is uninitilized
    self.make_deref(_box.clone(), Mode::Drop, guard.clone());

    self.top().cur_state.remove_pointer(_box.clone());

    let pointer_ident = self.ctx.pointer_ident(_box);
    let alloc_array =
      self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
    let index =
      self.ctx.index(alloc_array, pointer_ident, Type::bool_type());
    self.assign(index, self.ctx._false(), guard.clone());
  }

  /// Drop a struct may drop the inner box pointer
  fn drop_struct(&mut self, st: Expr, guard: Expr) {
    let def = st.ty().struct_def();
    for (i, (_, ty)) in def.1.iter().enumerate() {
      if !ty.is_box() && !ty.is_struct() { continue; }
      let object = 
        self.ctx.object(
          self.ctx.index(
            st.clone(),
            self.ctx.constant_usize(i),
            *ty),
          st.extract_ownership() 
        );
      self.symex_drop_rec(object, guard.clone());
    }
  }
}