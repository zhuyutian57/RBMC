
use num_bigint::BigInt;
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::Type;
use crate::program::program::bigint_to_usize;
use crate::symbol::symbol::*;
use crate::NString;
use super::symex::Symex;
use super::value_set::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
  Read,
  Drop,
  Dealloc,
  Slice(Option<usize>, Option<usize>),
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

    for elem in place.projection.iter() {
      ret =
        match elem {
          ProjectionElem::Deref
            => self
                .project_deref(
                  ret.clone(),
                  Mode::Read,
                  Guard::new(ctx.clone())
                ).unwrap(),
          ProjectionElem::Field(i, ty)
            => self.project_field(
                ret.clone(), 
                *i,
                Type::from(*ty)
              ),
          ProjectionElem::Index(local)
            => {
              let mut index =
                self
                  ._callback_symex
                  .exec_state
                  .current_local(*local, Level::Level2);
              self._callback_symex.rename(&mut index);
              self.project_index(ret.clone(), index)
            },
          ProjectionElem::ConstantIndex {
            offset,
            min_length,
            from_end 
          } => {
              let i = if *from_end { min_length - offset } else { *offset };
              let index = ctx.constant_usize(i as usize);
              self.project_index(ret.clone(), index)
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
    guard: Guard
  ) -> Option<Expr> {
    assert!(pt.ty().is_any_ptr());
    
    let mut objects = ObjectSet::new();
    self
      ._callback_symex
      .top()
      .cur_state
      .get_value_set(pt.clone(), &mut objects);

    let ctx = pt.ctx.clone();
    
    let mut ret = None;
    
    for (object, offset) in objects {
      // An object is valid if it is owned by some variable
      // according to the Ownership rule of Rust.
      if object.is_null_object() {
        self.dereference_null(pt.clone(), guard.clone(), mode);
        continue;
      }

      if object.is_unknown() {
        self.dereference_invalid_ptr(pt.clone(), mode, guard.clone());
        continue;
      }

      let mut pointer_cond =
        ctx.same_object(
          pt.clone(),
          ctx.address_of(object.clone(), pt.ty())
        );
      self._callback_symex.rename(&mut pointer_cond);
      let pointer_guard = Guard::from(pointer_cond);

      // Valid check
      let root_object = self.get_root_object(&object);
      let place_state =
        self
          ._callback_symex
          .exec_state
          .get_place_state(&root_object);
      if place_state.is_unknown() {
        self.valid_check(object.clone(), pointer_guard.clone());
      }
      
      if mode == Mode::Drop || mode == Mode::Dealloc {
        if let Some(x) = offset {
          if x == BigInt::ZERO { continue; }
          let msg = 
            format!(
              "{} {object:?} fail: the offset is {x} != 0",
              format!("{mode:?}").to_lowercase()
            ).into();
          self._callback_symex.claim(msg, pointer_guard.to_expr());
        }
        continue;
      }

      let final_offset =
        match offset {
          Some(x) => Some(x),
          None => {
            // The pointer only access a field of the object
            if pt.ty().pointee_ty() != object.ty() {
              Some(BigInt::ZERO)
            } else {
              None
            }
          },
        };

      let new_ret =
        self.build_ret(object, final_offset, mode, pointer_guard.clone());
      if new_ret == None { continue; }

      ret =
        match ret {
          Some(x) => {
            let cond = pointer_guard.to_expr();
            Some(ctx.ite(cond, new_ret.unwrap(), x))
          },
          None => new_ret,
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
    if object.ty().is_box() {
      // `box` performs as a special pointer. Use it directly.
      assert!(field == 0);
      return object;
    }

    assert!(object.ty().is_struct());
    let offset = self._callback_symex.ctx.constant_usize(field);
    self.build_with_const_offset(
      object,
      BigInt::from(field),
      self._callback_symex.ctx._true().into(),
      false
    ).unwrap()
  }

  /// Visit an array/slice. Return `Index(array/slice, i)`.
  fn project_index(
    &mut self,
    object: Expr,
    index: Expr
  ) -> Expr {
    let ty = object.ty();
    assert!(ty.is_array() || ty.is_slice());

    if index.is_constant() {
      let offset = index.extract_constant().to_integer();
      self.build_with_const_offset(
        object,
        offset,
        self._callback_symex.ctx._true().into(),
        false
      ).unwrap()
    } else {
      todo!("Non-const index")
    }
  }

  fn get_root_object(&mut self, object: &Expr) -> Expr {
    if object.extract_inner_expr().is_slice() {
      let inner_object = object.extract_inner_expr().extract_object();
      self.get_root_object(&inner_object)
    } else {
      object.clone()
    }
  }

  fn build_ret(
    &mut self,
    object: Expr,
    offset: Option<BigInt>,
    mode: Mode,
    guard: Guard
  ) -> Option<Expr> {
    match mode {
      Mode::Read
        => self.build_read(object, offset, guard),
      Mode::Slice(l, r)
        => self.build_slice(object, offset, l, r, guard),
      _ => todo!(),
    }
  }

  fn build_read(
    &mut self,
    object: Expr,
    offset: Option<BigInt>,
    guard: Guard
  ) -> Option<Expr> {
    if let Some(x) = offset {
      self.build_with_const_offset(object, x, guard, true)
    } else {
      Some(object.extract_inner_expr())
    }
  }

  fn build_with_const_offset(
    &mut self,
    object: Expr,
    offset: BigInt,
    guard: Guard,
    bound_check: bool
  ) -> Option<Expr> {
    let index =
      self
        ._callback_symex
        .ctx
        .constant_integer(offset.clone(), Type::isize_type());
    if bound_check {
      self.bound_check(object.clone(), index.clone(), guard);
    }
    
    let ty = 
      if object.ty().is_array() {
        object.ty().array_range()
      } else if object.ty().is_slice() {
        object.ty().slice_elem_ty()
      } else if object.ty().is_struct() {
        let i = bigint_to_usize(&offset);
        object.ty().struct_def().1[i].1
      } else {
        todo!("Not suport build {:?} with offset", object.ty())
      };
    Some(self._callback_symex.ctx.index(object, index, ty))
  }

  fn build_slice(
    &mut self,
    object: Expr,
    offset: Option<BigInt>,
    l: Option<usize>,
    r: Option<usize>,
    guard: Guard
  ) -> Option<Expr> {
    let inner_expr = object.extract_inner_expr();
    assert!(object.ty().is_array() || inner_expr.is_slice());
    let ctx = object.ctx.clone();
    let (root_object, start, len) =
      if object.ty().is_array() {
        let start =
          match offset { Some(o) => o, None => BigInt::ZERO, };
        let len = object.ty().array_size().expect("array must has len");
        (object, start, BigInt::from(len as usize))
      } else {
        let root_object = inner_expr.extract_object();
        let start =
          inner_expr.extract_slice_start().extract_constant().to_integer();
        let len =
          inner_expr.extract_slice_len().extract_constant().to_integer();
        (root_object, start, len)
      };
    
    let end = start.clone() + len.clone();
    let new_start =
      start.clone() + BigInt::from(match l { Some(s) => s, _ => 0 });
    let new_end =
      match r {
        Some(e) => start.clone() + BigInt::from(e),
        None => end.clone(),
      };
    let new_len = new_end.clone() - new_start.clone();

    if new_start > new_end || new_end > end || new_len < BigInt::ZERO {
      let msg =
        NString::from(
          format!(
            "slicing fail: [{:?}, {:?}) must be in {:?}[{:?}, {:?})",
            &new_start, &new_end, root_object, &start, &end
          )
        );
      self._callback_symex.claim(msg, ctx._false().into());
      None
    } else {
      let slice_start = ctx.constant_integer(new_start, Type::usize_type());
      let slice_len = ctx.constant_integer(new_len, Type::usize_type());
      Some(ctx.slice(root_object, slice_start, slice_len))
    }
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

  fn valid_check(&mut self, object: Expr, guard: Guard) {
    assert!(object.is_object());
    let ctx = object.ctx.clone();
    let mut invalid = ctx.invalid(object.clone());
    let msg =
      NString::from(format!("valid check: {object:?} is not alloced"));
    let mut error = guard.clone();
    error.add(invalid);
    self._callback_symex.claim(msg, error.to_expr());
  }

  fn bound_check(&mut self, object: Expr, index: Expr, guard: Guard) {
    assert!(object.is_object());
    let ctx = object.ctx.clone();
    let array_ty = object.ty();
    let s = array_ty.array_size();
    if let Some(len) = s {
      let mut out_of_bound =
        ctx.or(
          ctx.lt(index.clone(), ctx.constant_isize(0)),
          ctx.ge(index.clone(), ctx.constant_isize(len as isize))
        );
      self._callback_symex.rename(&mut out_of_bound);
      out_of_bound.simplify();
      let msg =
        NString::from(format!("bound check: {object:?}[{index:?}] is out-of-bound"));
      let mut error = guard.clone();
      error.add(out_of_bound);
      self._callback_symex.claim(msg, error.to_expr());
    }
  }

  fn dereference_null(&mut self, pt: Expr, mut guard: Guard, mode: Mode) {
    assert!(pt.ty().is_any_ptr());
    assert!(mode == Mode::Read);
    let ctx = pt.ctx.clone();
    let null = ctx.null(pt.ty());
    let msg =
      NString::from(format!("dereference failure: null pointer dereference"));
    let mut is_null = ctx.eq(pt, null);
    self._callback_symex.rename(&mut is_null);
    let mut error = guard.clone();
    error.add(is_null);
    self._callback_symex.claim(msg, error.to_expr());
  }

  fn dereference_invalid_ptr(&mut self, pt: Expr, mode: Mode, mut guard: Guard) {
    // Check the pointer is invalid
    let ctx = pt.ctx.clone();
    let pointer_base = ctx.pointer_base(pt.clone());
    let not_null = ctx.ne(pt.clone(), ctx.null(pt.ty()));
    let alloc_array =
      self
        ._callback_symex
        .exec_state
        .ns
        .lookup_object(NString::ALLOC_SYM);
    let mut not_alloced =
      ctx.not(
        ctx.index(
          alloc_array,
          pointer_base,
          Type::bool_type()
        )
      );
    self._callback_symex.rename(&mut not_alloced);
    let msg =
      match mode {
        Mode::Read => NString::from("dereference failure: invalid pointer"),
        Mode::Drop => NString::from("drop failure: uninitilized box(smart) pointer"),
        Mode::Dealloc => NString::from("dealloc failure: invalid pointer"),
        _ => todo!(),
      };
    let mut error = guard.clone();
    error.add(not_null);
    error.add(not_alloced);
    self._callback_symex.claim(msg, error.to_expr());
  }
}