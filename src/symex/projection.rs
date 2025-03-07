
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::symbol::symbol::*;
use crate::NString;
use super::symex::Symex;
use super::value_set::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
  Read,
  Drop,
  Dealloc,
}

pub(super) struct Projection<'a, 'cfg> {
  _callback_symex: &'a mut Symex<'cfg>,
}

impl<'a, 'cfg> Projection<'a, 'cfg> {
  pub(super) fn new(state: &'a mut Symex<'cfg>) -> Self {
    Projection { _callback_symex: state }
  }

  pub(super) fn project(&mut self, place: &Place) -> Expr {
    let ctx = self._callback_symex.ctx.clone();

    let mut ret =
      self
        ._callback_symex
        .exec_state
        .current_local(place.local, Level::Level1);
    ret = ctx.object(ret);

    for elem in place.projection.iter() {
      ret =
        match elem {
          ProjectionElem::Deref
            => self
                .project_deref(ret.clone(), Mode::Read, ctx._true()).unwrap(),
          ProjectionElem::Field(i, ty)
            => self.project_field(ret.clone(), *i, Type::from(*ty)),
          ProjectionElem::Index(local)
            => {
              let index =
                self
                  ._callback_symex
                  .exec_state
                  .current_local(*local, Level::Level2);
              self.project_index(ret.clone(), index, true)
            },
          ProjectionElem::ConstantIndex {
            offset,
            min_length,
            from_end }
            => {
              let i = if *from_end { min_length - offset } else { *offset };
              let index = ctx.constant_usize(i as usize);
              self.project_index(ret.clone(), index, false)
            },
          _ => panic!("Not support {elem:?} for {ret:?}"),
        };
    }

    ret
  }

  /// Dereferencing raw pointer/reference/box pointer.
  /// Return the objects it points to.
  pub(super) fn project_deref(
    &mut self,
    pt: Expr,
    mode: Mode,
    guard: Expr,
  ) -> Option<Expr> {
    assert!(pt.ty().is_any_ptr());
    
    let mut objects = ObjectSet::new();
    self
      ._callback_symex
      .exec_state
      .cur_state()
      .get_value_set(pt.clone(), &mut objects);

    let ctx = pt.ctx.clone();
    
    let mut ret = None;

    // Dereferencing a null pointer
    if objects.contains(&ctx.null_object(pt.ty().pointee_ty())) {
      // box(smart) pointer is not null
      assert!(mode == Mode::Read);
      self.dereference_null(pt.clone(), guard.clone());
    }
    
    // The pointer contains nothing. Returning invalid object
    if objects
        .iter()
        .fold(false, |acc, x| acc | x.is_unknown()) {
      self.dereference_invalid_ptr(pt.clone(), mode, guard.clone());
      ret = 
        match mode {
          Mode::Read => Some(self.make_invalid_object(pt.ty().pointee_ty())),
          _ => None,
        };
    }

    if mode == Mode::Drop || mode == Mode::Dealloc { return ret; }
    
    for object in objects{
      // An object is valid if it is owned by some variable
      // according to the Ownership rule of Rust.
      if object.is_null_object() ||
         object.is_unknown() { continue; }

      // Do more analysis
      let place_state =
        self
          ._callback_symex
          .top_mut()
          .cur_state
          .place_states
          .place_state(&object);
      if !place_state.is_own() && !place_state.is_alloced() {
        let pointer_guard =
          ctx.same_object(
            pt.clone(),
            ctx.address_of(object.clone(), pt.ty())
          );
        self.valid_check(object.clone(), pointer_guard);
      }

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

    match ret {
      Some(_) => ret,
      None => Some(self.make_invalid_object(pt.ty().pointee_ty())),
    }
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
    let i = ctx.constant_usize(field);
    
    let index = ctx.index(object, i, Type::from(ty));
    ctx.object(index)
  }

  /// Visit an array/slice. Return `Index(array/slice, i)`.
  fn project_index(
    &mut self,
    object: Expr,
    index: Expr,
    bound_check: bool
  ) -> Expr {
    let array_ty = object.ty();
    assert!(array_ty.is_array());
    
    // Bound check
    if bound_check {
      self.bound_check(object.clone(), index.clone());
    }

    let ctx = object.ctx.clone();
    let index = ctx.index(object, index, array_ty.array_range());
    ctx.object(index)
  }

  fn make_invalid_object(&mut self, ty: Type) -> Expr {
    let ctx = self._callback_symex.ctx.clone();
    let l0_symbol =
      self
        ._callback_symex
        .exec_state
        .l0_symbol(NString::INVALID_OBJECT, ty);
    let l1_symbol =
      self
        ._callback_symex
        .exec_state
        .new_symbol(&l0_symbol, Level::Level1);
    ctx.object(l1_symbol)
  }

  fn valid_check(&mut self, object: Expr, guard: Expr) {
    assert!(object.is_object());
    let ctx = object.ctx.clone();
    let invalid = ctx.invalid(object.clone());
    let msg =
      NString::from(format!("dereference failure: {object:?} is not alloced"));
    self._callback_symex.claim(msg, ctx.and(guard, invalid));
  }

  fn bound_check(&mut self, object: Expr, index: Expr) {
    let ctx = object.ctx.clone();
    let array_ty = object.ty();
    let s = array_ty.array_size();
    if let Some(len) = s {
      // If the array/slice is finite, the assertion must be
      // in the mir. No need to check again
      if len > 0 { return; }
      let out_of_bound =
        ctx.ge(index.clone(), ctx.constant_usize(len as usize));
      let msg =
        NString::from(format!("bound check: {object:?}[{index:?}] is out-of-bound"));
      self._callback_symex.claim(msg, out_of_bound);
    }
  }

  fn dereference_null(&mut self, pt: Expr, guard: Expr) {
    assert!(pt.ty().is_any_ptr());
    let ctx = pt.ctx.clone();
    let null = ctx.null(pt.ty());
    let msg =
      NString::from(format!("dereference failure: null pointer dereference"));
    let is_deref_null = ctx.and(guard, ctx.eq(pt, null));
    self._callback_symex.claim(msg, is_deref_null);
  }

  fn dereference_invalid_ptr(&mut self, pt: Expr, mode: Mode, guard: Expr) {
    // Check the pointer is invalid
    let ctx = pt.ctx.clone();
    let pointer_ident = ctx.pointer_ident(pt.clone());
    let ne = ctx.ne(pt.clone(), ctx.null(pt.ty()));
    let alloc_array =
      self
        ._callback_symex
        .exec_state
        .ns
        .lookup_object(NString::ALLOC_SYM);
    let index =
      ctx.index(alloc_array, pointer_ident, Type::bool_type());
    let msg =
      match mode {
        Mode::Read => NString::from("dereference failure: invalid pointer"),
        Mode::Drop => NString::from("drop failure: uninitilized box(smart) pointer"),
        Mode::Dealloc => NString::from("dealloc failure: invalid pointer"),
      };
    let fail =
      ctx.and(ctx.and(guard, ne), ctx.not(index));
    self._callback_symex.claim(msg, fail);
  }

}