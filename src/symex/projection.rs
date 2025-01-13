
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::symbol::symbol::*;
use super::exec_state::*;
use super::value_set::*;

/// Dereferencing a place
pub(super) struct Projector<'sym, 'exec> {
  _callback_state: &'sym mut ExecutionState<'exec>,
}

impl<'sym, 'exec> Projector<'sym, 'exec> {
  pub fn new(state: &'sym mut ExecutionState<'exec>) -> Self {
    Projector { _callback_state: state }
  }

  #[inline]
  fn state_mut(&mut self) -> &mut ExecutionState<'exec> {
    &mut self._callback_state
  }

  /// TODO: add assertion for dereference
  pub fn project(&mut self, place: &Place) -> Expr {

    let mut ret = self.state_mut().current_local(place.local, Level::Level1);

    let ctx = ret.ctx.clone();

    for elem in place.projection.iter() {
      ret =
        match elem {
          ProjectionElem::Deref
            => Some(self.project_deref(ret.clone())),
          ProjectionElem::Field(i, ty)
            => {
              if ret.ty().is_box() && *i == 0 {
                // `box` performs as a special raw pointer.
                // Use it directly, instead of projection
                Some(ret)
              } else {
                let mut object = ret;
                if !object.is_object() { object = ctx.object(object); }
                Some(ctx.index_of(object, *i, Type::from(*ty)))
              }
            },
          _ => None,
        }.expect(format!("{elem:?}").as_str());
    }

    ret
  }

  fn project_deref(&mut self, expr: Expr) -> Expr {
    let mut objects = ObjectSet::new();
    self.state_mut().cur_state().get_value_set(expr.clone(), &mut objects);
    
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

    ret.expect(format!("*{expr:?}").as_str())
  }
}