
use num_bigint::BigInt;

use crate::{expr::expr::*, symbol::symbol::Level};
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  /// `Move` semantic: if a value is move, it becomes uninitialized.
  pub(super) fn symex_move(&mut self, expr: Expr) {
    if let Some(sub_exprs) = expr.sub_exprs() {
      for e in sub_exprs {
        self.symex_move(e);
      }
    }

    if expr.is_move() {
      let object = expr.extract_object();
      self.move_rec(object);
    }
  }

  fn move_rec(&mut self, expr: Expr) {
    if expr.ty().is_any_ptr() {
      self.top_mut().cur_state.remove_pointer(expr);
      return; 
    }

    if expr.is_symbol() {
      // A new l2 variable in SSA means fresh value
      self.exec_state.new_symbol(&expr, Level::Level2);
      return;
    }

    if expr.ty().is_struct() {
      let def = expr.ty().struct_def();
      for (i, (_, ty)) in def.1.iter().enumerate() {
        if !ty.is_box() && !ty.is_struct() { continue; }
        let index = 
          self.ctx.index(
            expr.clone(),
            self.ctx.constant_isize(BigInt::from(i)),
            *ty
          );
        self.move_rec(index);
      }
      return;
    } 
  }

}