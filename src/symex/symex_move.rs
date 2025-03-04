
use stable_mir::mir::Place;

use crate::expr::expr::*;
use crate::expr::ty::*;
use super::place_state::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  /// `Move` semantic: if a value is move, it becomes uninitialized.
  pub(super) fn symex_move(&mut self, place: &Place) -> Expr {
    let expr = self.make_project(place);
    self.exec_state.update_place_state(
      expr.clone(),
      PlaceState::Moved
    );
    self.move_rec(expr.clone());
    expr
  }

  fn move_rec(&mut self, expr: Expr) {
    if expr.ty().is_box() {
      self.top().cur_state.remove_pointer(expr);
      return; 
    }

    if expr.ty().is_struct() {
      let def = expr.ty().struct_def();
      for (i, (_, ty)) in def.1.iter().enumerate() {
        if !ty.is_box() && !ty.is_struct() { continue; }
        let index = 
          self.ctx.index(
            expr.clone(),
            self.ctx.constant_usize(i),
            *ty
          );
        self.move_rec(index);
      }
    }

    // Maybe more 
  }

}