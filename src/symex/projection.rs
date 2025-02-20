
use stable_mir::mir::*;
use stable_mir::ty::UintTy;

use crate::expr::constant::BigInt;
use crate::expr::expr::*;
use crate::expr::predicates::*;
use crate::expr::ty::Type;
use crate::symbol::symbol::*;
use crate::vc::vc::VCSystem;
use super::exec_state::*;
use super::symex::Symex;
use super::value_set::*;

/// Dereferencing a place
pub(super) struct Projector<'a, 'sym> {
  _callback_symex: &'a mut Symex<'sym>,
}

impl<'a, 'sym> Projector<'a, 'sym> {
  pub fn new(state: &'a mut Symex<'sym>) -> Self {
    Projector { _callback_symex: state }
  }

  /// TODO: add assertion for dereference
  pub fn project(&mut self, place: &Place) -> Expr {
    let mut ret =
      self
        ._callback_symex
        .exec_state
        .current_local(place.local, Level::Level1);
    let ctx = ret.ctx.clone();
    ret = ctx.object(ret, Ownership::Own);

    for elem in place.projection.iter() {
      ret =
        match elem {
          ProjectionElem::Deref
            => Some(self.project_deref(ret.clone())),
          ProjectionElem::Field(i, ty)
            => {
              if ret.ty().is_box() && *i == 0 {
                // `box` performs as a special raw pointer. Use it directly.
                Some(ret)
              } else {
                let index =
                  ctx.constant_integer(
                    BigInt(false, *i as u128),
                    Type::unsigned_type(UintTy::Usize)
                  );
                let load = ctx.index(ret, index.clone(), Type::from(*ty));
                let ownership = load.extract_ownership();
                Some(ctx.object(load, ownership))
              }
            },
          _ => None,
        }.expect(format!("{elem:?}").as_str());
    }

    ret
  }

  fn project_deref(&mut self, expr: Expr) -> Expr {
    let mut objects = ObjectSet::new();
    self
      ._callback_symex
      .exec_state
      .cur_state()
      .get_value_set(expr.clone(), &mut objects);
    
    let ctx = expr.ctx.clone();

    for object in objects.iter() {
      if object.extract_ownership().is_own() { continue; }
      // assertion for invalid dereference
      // use uninterpreted function?
    }

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