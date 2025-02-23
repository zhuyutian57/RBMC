
use stable_mir::mir::*;
use stable_mir::ty::UintTy;

use crate::expr::constant::BigInt;
use crate::expr::expr::*;
use crate::expr::predicates::*;
use crate::expr::ty::Type;
use crate::symbol::symbol::*;
use crate::vc::vc::VCSystem;
use crate::NString;
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

  pub fn project(&mut self, place: &Place) -> Expr {
    let mut ret =
      self
        ._callback_symex
        .exec_state
        .current_local(place.local, Level::Level1);
    ret = ret.ctx.clone().object(ret, Ownership::Own);

    for elem in place.projection.iter() {
      ret =
        match elem {
          ProjectionElem::Deref
            => self.project_deref(ret.clone()),
          ProjectionElem::Field(i, ty)
            => self.project_field(
              ret.clone(),
              *i,
              Type::from(*ty)),
          ProjectionElem::Index(local)
            => {
              let index =
                self
                  ._callback_symex
                  .exec_state
                  .current_local(*local, Level::Level2);
              self.project_index(ret.clone(), index)
            },
          _ => panic!("Not support {elem:?} for {ret:?}"),
        };
    }

    ret
  }

  /// Dereferencing raw pointer/reference/box pointer.
  /// Return the objects it points to.
  fn project_deref(&mut self, pt: Expr) -> Expr {
    assert!(pt.ty().is_any_ptr());
    let mut objects = ObjectSet::new();
    self
      ._callback_symex
      .exec_state
      .cur_state()
      .get_value_set(pt.clone(), &mut objects);
    
    let ctx = pt.ctx.clone();

    if objects.is_empty() {
      todo!()
    }

    for object in objects.iter() {
      // An object is valid if it is owned by some variable
      // according to the Ownership rule of Rust.
      if object.extract_ownership().is_own() { continue; }
      
      let guard =
        ctx.same_object(
          pt.clone(),
          ctx.address_of(object.clone(), pt.ty())
        );
      self.valid_check(object.clone(), guard);
    }

    let mut ret = None;
    for object in objects {
      ret =
        match ret {
          Some(x) => {
            let obj_adr = ctx.address_of(object.clone(), pt.ty());
            let cond = ctx.same_object(pt.clone(), obj_adr);
            Some(ctx.ite(cond, object.clone(), x))
          },
          None => Some(object),
        }
    }

    ret.expect(format!("*{pt:?}").as_str())
  }

  /// Visit a field of a struct. Return `Index(object, i)`.
  /// Note that the special visit for box pointer.
  fn project_field(&mut self, object: Expr, field: usize, ty: Type) -> Expr {
    if object.ty().is_box() && field == 0 {
      // `box` performs as a special raw pointer. Use it directly.
      return object;
    }

    assert!(object.ty().is_struct());

    let ctx = object.ctx.clone();

    // TODO: shall we add bound check here?

    let field =
      ctx.constant_integer(
        BigInt(false, field as u128),
        Type::unsigned_type(UintTy::Usize)
      );
    
    let index = ctx.index(object, field, Type::from(ty));
    let ownership = index.extract_ownership();
    ctx.object(index, ownership)
  }

  /// Visit an array/slice. Return `Index(array/slice, i)`.
  fn project_index(&mut self, object: Expr, index: Expr) -> Expr {
    todo!()
  }

  fn valid_check(&mut self, object: Expr, guard: Expr) {
    assert!(object.is_object());
    let ctx = object.ctx.clone();
    let invalid = ctx.invalid(object.clone());
    self.dereference_failure(
      NString::from(format!("dereference failure: {object:?} is not alloced")),
      invalid,
      guard
    );
  }

  fn dereference_failure(&mut self, msg: NString, vc: Expr, guard: Expr) {
    let ctx = vc.ctx.clone();

    if vc.is_invalid() {
      let object = vc.extract_object();
      let ptr_indent =
        ctx
          .pointer_ident(
            ctx.address_of(
              object.clone(),
              object.extract_address_type()
            )
          );
      let alloc_array = self._callback_symex.lookup(NString::ALLOC_SYM);
      let mut is_not_alloced =
          ctx.not(
            ctx.index(
              alloc_array,
              ptr_indent,
              Type::bool_type()
            )
          );
      self._callback_symex.exec_state.rename(&mut is_not_alloced, Level::Level2);
      self
        ._callback_symex
        .vc_system
        .borrow_mut()
        .assert(msg, is_not_alloced);
      return;
    }

    panic!("Not support {msg:?}\n{vc:?}");
  }
}