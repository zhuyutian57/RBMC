use num_bigint::BigInt;
use stable_mir::mir::*;
use stable_mir::ty::IndexedVal;

use super::place_state::PlaceState;
use super::symex::Symex;
use super::value_set::*;
use crate::expr::context::ExprCtx;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::Type;
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::NString;
use crate::symbol::symbol::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Read,
    Drop,
    Dealloc,
}

pub(super) struct Projection<'a, 'cfg> {
    _ctx: ExprCtx,
    _callback_symex: &'a mut Symex<'cfg>,
}

impl<'a, 'cfg> Projection<'a, 'cfg> {
    pub(super) fn new(symex: &'a mut Symex<'cfg>) -> Self {
        Projection { _ctx: symex.ctx.clone(), _callback_symex: symex }
    }

    pub(super) fn project(&mut self, place: &Place) -> Expr {
        let mut ret = self._callback_symex.exec_state.current_local(place.local, Level::Level1);

        for elem in place.projection.iter() {
            ret = match elem {
                ProjectionElem::Deref => self.project_deref(
                    ret.clone(),
                    Mode::Read,
                    Guard::new(self._ctx.clone()),
                    ret.ty().pointee_ty(),
                ),
                ProjectionElem::Field(i, ty) => self.project_field(ret.clone(), *i, Type::from(ty)),
                ProjectionElem::Index(local) => {
                    let mut index =
                        self._callback_symex.exec_state.current_local(*local, Level::Level1);
                    self._callback_symex.rename(&mut index);
                    self.project_index(ret.clone(), index)
                }
                ProjectionElem::ConstantIndex { offset, min_length, from_end } => {
                    let i = if *from_end { *min_length - *offset } else { *offset };
                    let index = self._ctx.constant_usize(i as usize);
                    self.project_index(ret.clone(), index)
                }
                ProjectionElem::Downcast(i) => {
                    assert!(ret.ty().is_enum());
                    let idx = i.to_index();
                    self._ctx.as_variant(ret, self._ctx.constant_usize(idx))
                }
                _ => panic!("Not support {elem:?} for {ret:?}"),
            };
        }

        ret
    }

    /// Dereferencing raw pointer/reference/box pointer.
    /// Return the objects it points to.
    pub(super) fn project_deref(&mut self, pt: Expr, mode: Mode, guard: Guard, ty: Type) -> Expr {
        assert!(pt.ty().is_primitive_ptr());

        let mut objects = ObjectSet::new();
        self._callback_symex.top().cur_state.get_value_set(pt.clone(), &mut objects);

        let mut ret = None;

        if objects.iter().fold(false, |acc, (x, _)| acc | x.is_null_object()) {
            self.dereference_null(pt.clone(), guard.clone(), mode);
        }

        if objects.iter().fold(false, |acc, (x, _)| acc | x.is_unknown()) {
            self.dereference_invalid_ptr(pt.clone(), mode, guard.clone());
        }

        for (object, offset) in
            objects.into_iter().filter(|(x, _)| !x.is_null_object() && !x.is_unknown())
        {
            // Note that all pointer is constructed from a root object.
            // The root object here is used to retrieve place states.
            let root_object = object.extract_root_object();
            let mut pointer_cond = self._ctx.same_object(
                pt.clone(),
                self._ctx.address_of(root_object.clone(), root_object.extract_address_type()),
            );
            self._callback_symex.rename(&mut pointer_cond);
            let pointer_guard = Guard::from(pointer_cond);

            // Valid check
            let place_state = self._callback_symex.exec_state.get_place_state(&root_object);
            if place_state.is_unknown() || place_state.is_dead() {
                self.valid_check(root_object.clone(), place_state, mode, pointer_guard.clone());
            }

            if mode == Mode::Drop || mode == Mode::Dealloc {
                self.dealloc_check(object.clone(), offset, ty, mode, pointer_guard.clone());
                continue;
            }

            let new_ret = self.build_ret(pt.clone(), object, offset, pointer_guard.clone());

            if new_ret == None {
                continue;
            }

            ret = match ret {
                Some(x) => {
                    let cond = pointer_guard.to_expr();
                    Some(self._ctx.ite(cond, new_ret.unwrap(), x))
                }
                None => new_ret,
            }
        }

        match ret {
            Some(x) => x,
            None => self._ctx.invalid_object(pt.ty().pointee_ty()),
        }
    }

    /// Visit a field of a struct. Return `Index(object, i)`.
    ///
    /// TODO: Add bound check. The projection may fail is the pointer is casted by raw pointer.
    fn project_field(&mut self, object: Expr, field: usize, ty: Type) -> Expr {
        assert!(object.ty().is_struct() || object.ty().is_tuple() || object.is_as_variant());
        if object.is_invalid_object() {
            return self._ctx.invalid_object(ty);
        }
        self.build_with_const_offset(object, BigInt::from(field), ty)
    }

    /// Visit an array/slice. Return `Index(array/slice, i)`.
    ///
    /// TODO: Add bound check. The projection may fail is the pointer is casted by raw pointer.
    fn project_index(&mut self, object: Expr, index: Expr) -> Expr {
        let ty = object.ty();
        assert!(ty.is_array() || ty.is_slice());
        let elem_ty = ty.elem_type();

        if object.is_invalid_object() {
            return self._ctx.invalid_object(elem_ty);
        }

        if index.is_constant() {
            let offset = index.extract_constant().to_integer();
            self.build_with_const_offset(object, offset, elem_ty)
        } else {
            todo!("Non-const index")
        }
    }

    /// Build `expr` for dereference according to the pointee type.
    fn build_ret(
        &mut self,
        pt: Expr,
        object: Expr,
        offset: Option<BigInt>,
        guard: Guard,
    ) -> Option<Expr> {
        if pt.ty().pointee_ty() == object.ty() {
            // Access the whole object
            assert!(object.ty().is_slice() || offset == None || offset == Some(BigInt::ZERO));
            Some(object.extract_inner_expr())
        } else if pt.ty().is_slice_ptr() {
            // Build slice
            self.build_slice(pt, object, offset)
        } else {
            // Access one element of an array/slice, or a field of a struct/tuple
            self.build_index(object, offset, pt.ty().pointee_ty(), guard)
        }
    }

    /// Slicing will be check by `core::slice::*`, do not need to check here?
    /// TODO: fix more situations
    fn build_slice(&mut self, pt: Expr, object: Expr, offset: Option<BigInt>) -> Option<Expr> {
        // Since `pt` points to `&object + offset`, we compute the `len` of generated slice.
        let (root_object, mut start) = if object.ty().is_array() {
            (object, BigInt::ZERO)
        } else {
            assert!(object.extract_inner_expr().is_slice());
            let inner_expr = object.extract_root_object();
            let start = inner_expr.extract_slice_start().extract_constant().to_integer();
            (inner_expr.extract_root_object(), start)
        };
        assert!(root_object.ty().is_array());
        if let Some(x) = offset {
            if x < BigInt::ZERO {
                return None;
            }
            start += x;
        }
        let start = self._ctx.constant_usize(bigint_to_usize(&start));
        let len = self._ctx.pointer_meta(pt);
        Some(self._ctx.slice(root_object, start, len))
    }

    fn build_index(
        &mut self,
        object: Expr,
        offset: Option<BigInt>,
        ty: Type,
        guard: Guard,
    ) -> Option<Expr> {
        let i = self._ctx.constant_integer(
            match offset {
                Some(x) => x,
                _ => BigInt::ZERO,
            },
            Type::isize_type(),
        );
        let out_of_bound = self.bound_check(object.clone(), i.clone(), guard);
        // TODO: check alignment
        if out_of_bound == Some(true) { None } else { Some(self._ctx.index(object, i, ty)) }
    }

    /// Notice that `offset` is in field-level
    fn build_with_const_offset(&mut self, object: Expr, offset: BigInt, ty: Type) -> Expr {
        let i = bigint_to_usize(&offset);
        let index = self._ctx.constant_usize(i);
        let new_object = if object.is_object() { object } else { self._ctx.object(object) };
        self._ctx.index(new_object, index, ty)
    }

    fn valid_check(&mut self, object: Expr, state: PlaceState, mode: Mode, guard: Guard) {
        assert!(object.is_object());
        let invalid =
            if state.is_unknown() { self._ctx.invalid(object.clone()) } else { self._ctx._true() };
        let msg = match mode {
            Mode::Read => format!("dereference failure: {object:?} is dead").into(),
            Mode::Dealloc | Mode::Drop => {
                format!("{} failure: {object:?} is dead", format!("{mode:?}").to_lowercase()).into()
            }
        };
        let mut error = guard.clone();
        error.add(invalid);
        self._callback_symex.claim(msg, error.to_expr());
    }

    /// Bound check is in field-level.
    fn bound_check(&mut self, object: Expr, offset: Expr, guard: Guard) -> Option<bool> {
        assert!(object.is_object());
        let ty = object.ty();
        if ty.is_enum() { return None; }
        let s = if ty.is_array() {
            ty.array_len()
        } else if ty.is_struct() || ty.is_tuple() {
            Some(ty.fields())
        } else {
            todo!()
        };
        let mut res = None;
        if let Some(len) = s {
            let mut out_of_bound = self._ctx.or(
                self._ctx.lt(offset.clone(), self._ctx.constant_isize(0)),
                self._ctx.ge(offset.clone(), self._ctx.constant_isize(len as isize)),
            );
            self._callback_symex.rename(&mut out_of_bound);
            out_of_bound.simplify();
            // TODO: do more analysis
            if out_of_bound.is_true() {
                res = Some(true);
            }
            let msg = NString::from(format!("dereference failure: index out of array bound"));
            let mut error = guard.clone();
            error.add(out_of_bound);
            self._callback_symex.claim(msg, error.to_expr());
        } else {
            todo!();
        }
        res
    }

    fn dereference_null(&mut self, pt: Expr, guard: Guard, mode: Mode) {
        assert!(pt.ty().is_primitive_ptr());
        let null = self._ctx.null(pt.ty());
        let msg = match mode {
            Mode::Read => "dereference failure: null pointer dereference".into(),
            Mode::Dealloc => "dealloc failure: dealloce a null pointer".into(),
            _ => todo!(),
        };
        let mut is_null = self._ctx.eq(pt, null);
        self._callback_symex.rename(&mut is_null);
        let mut error = guard.clone();
        error.add(is_null);
        self._callback_symex.claim(msg, error.to_expr());
    }

    fn dereference_invalid_ptr(&mut self, pt: Expr, mode: Mode, guard: Guard) {
        // Check the pointer is invalid
        let pointer_base = self._ctx.pointer_base(pt.clone());
        let not_null = self._ctx.ne(pt.clone(), self._ctx.null(pt.ty()));
        let ident = Ident::Global(NString::ALLOC_SYM);
        let alloc_array = self._callback_symex.exec_state.ns.lookup_object(ident);
        let mut not_alloced =
            self._ctx.not(self._ctx.index(alloc_array, pointer_base, Type::bool_type()));
        self._callback_symex.rename(&mut not_alloced);
        let msg = match mode {
            Mode::Read => NString::from("dereference failure: invalid pointer"),
            // TODO: support more smart pointer
            Mode::Drop => format!("drop failure: uninitilized {:?} pointer", pt.ty().name()).into(),
            Mode::Dealloc => NString::from("dealloc failure: invalid pointer"),
        };
        let mut error = guard.clone();
        error.add(not_null);
        error.add(not_alloced);
        self._callback_symex.claim(msg, error.to_expr());
    }

    fn dealloc_check(
        &mut self,
        object: Expr,
        offset: Option<BigInt>,
        ty: Type,
        mode: Mode,
        guard: Guard,
    ) {
        let object_ty = object.ty();
        // Offset check
        let tmp_object = if object_ty.is_primitive() || object_ty.is_primitive_ptr() {
            object.clone()
        } else {
            self._ctx.index(
                object.clone(),
                if let Some(x) = offset {
                    self._ctx.constant_integer(x, Type::isize_type())
                } else {
                    self._ctx.constant_integer(BigInt::ZERO, Type::isize_type())
                },
                ty,
            )
        };
        let total_offset = tmp_object.compute_bytes_offset();
        let msg = format!(
            "{} failure: the offset must be 0({total_offset:?} bytes != 0)",
            format!("{mode:?}").to_lowercase()
        )
        .into();
        let mut new_guard = guard.clone();
        let zero = self._ctx.constant_isize(0);
        new_guard.add(self._ctx.ne(total_offset, zero));
        self._callback_symex.claim(msg, new_guard.to_expr());

        // Check layout
        if object_ty != ty {
            let msg = format!(
                "{} failure: the layout is {ty:?} where {:?} is required",
                format!("{mode:?}").to_lowercase(),
                object_ty
            )
            .into();
            let mut new_guard = guard.clone();
            new_guard.add(self._ctx._true());
            self._callback_symex.claim(msg, new_guard.to_expr());
        }
    }
}
