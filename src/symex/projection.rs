
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::symbol::symbol::*;
use super::frame::*;
use super::value_set::ObjectSet;

/// Dereferencing a place
pub(super) struct Projector<'sym, 'frame> {
  frame: &'sym mut Frame<'frame>,
}

impl<'sym, 'frame> Projector<'sym, 'frame> {
  pub fn new(frame: &'sym mut Frame<'frame>) -> Self {
    Projector { frame }
  }

  /// TODO: add assertion for dereference
  pub fn project(&mut self, place: &Place) -> Expr {

    let mut ret = self.frame.current_local(place.local, Level::level1);

    for elem in place.projection.iter() {
      ret =
        match elem {
          ProjectionElem::Deref => Some(self.project_deref(ret.clone())),
          _ => None,
        }.expect("???");
    }

    ret
  }

  fn project_deref(&mut self, expr: Expr) -> Expr {
    let mut objects = ObjectSet::new();
    self.frame.cur_state().get_value_set(&expr, &mut objects);
    
    let ctx = expr.ctx.clone();

    let mut ret = None;
    for object in objects {
      ret =
        match ret {
          Some(x) => {
            let cond = ctx.same_object(expr.clone(), object.clone());
            Some(ctx.ite(cond, object.clone(), x))
          },
          None => Some(object),
        }
    }

    ret.expect("Fail dereference?")
  }

  
}