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
use crate::symbol::nstring::NString;
use crate::symbol::symbol::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Read,
    Drop,
    Dealloc,
    Slice(Option<usize>, Option<usize>),
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

        let mut project_elem = place.projection.clone();
        if !project_elem.is_empty() && ret.ty().is_smart_ptr() {
            // Remove project_index. Use box as a pointer
            if ret.ty().is_box() {
                project_elem.drain(0..2);
            }
            ret = self._ctx.inner_pointer(ret);
        }

        for elem in project_elem {
            ret = match elem {
                ProjectionElem::Deref => self.project_deref(
                    ret.clone(),
                    Mode::Read,
                    Guard::new(self._ctx.clone()),
                    ret.ty().pointee_ty(),
                ),
                ProjectionElem::Field(i, ty) => self.project_field(ret.clone(), i, Type::from(ty)),
                ProjectionElem::Index(local) => {
                    let mut index =
                        self._callback_symex.exec_state.current_local(local, Level::Level1);
                    self._callback_symex.rename(&mut index);
                    self.project_index(ret.clone(), index)
                }
                ProjectionElem::ConstantIndex { offset, min_length, from_end } => {
                    let i = if from_end { min_length - offset } else { offset };
                    let index = self._ctx.constant_usize(i as usize);
                    self.project_index(ret.clone(), index)
                }
                ProjectionElem::Downcast(i) => {
                    assert!(ret.ty().is_enum());
                    let idx = i.to_index();
                    let def = ret.ty().enum_def();
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
        assert!(pt.ty().is_any_ptr());

        let mut objects = ObjectSet::new();
        self._callback_symex.top().cur_state.get_value_set(pt.clone(), &mut objects);

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

            // Note that all pointer is constructed from a root object
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

            let new_ret = self.build_ret(object, offset, mode, pointer_guard.clone(), ty);
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
            None => self.make_invalid_object(pt.ty().pointee_ty()),
        }
    }

    /// Visit a field of a struct. Return `Index(object, i)`.
    fn project_field(&mut self, object: Expr, field: usize, ty: Type) -> Expr {
        assert!(object.ty().is_struct() || object.ty().is_tuple() || object.is_as_variant());
        self.build_with_const_offset(
            object,
            BigInt::from(field),
            self._ctx._true().into(),
            ty,
            false,
        )
        .unwrap()
    }

    /// Visit an array/slice. Return `Index(array/slice, i)`.
    fn project_index(&mut self, object: Expr, index: Expr) -> Expr {
        let ty = object.ty();
        assert!(ty.is_array() || ty.is_slice());
        let elem_ty = ty.elem_type();

        if index.is_constant() {
            let offset = index.extract_constant().to_integer();
            self.build_with_const_offset(object, offset, self._ctx._true().into(), elem_ty, false)
                .unwrap()
        } else {
            todo!("Non-const index")
        }
    }

    fn build_ret(
        &mut self,
        object: Expr,
        offset: Option<BigInt>,
        mode: Mode,
        guard: Guard,
        ty: Type,
    ) -> Option<Expr> {
        match mode {
            Mode::Read => self.build_read(object, offset, guard, ty),
            Mode::Slice(l, r) => self.build_slice(object, offset, l, r),
            _ => todo!(),
        }
    }

    fn build_read(
        &mut self,
        object: Expr,
        offset: Option<BigInt>,
        guard: Guard,
        ty: Type,
    ) -> Option<Expr> {
        // Compute the final offset of the accessing region.
        let final_offset = if object.ty() == ty || offset != None {
            // Access the whole object or the expr has arithmetic
            offset
        } else {
            // Access part of object
            let range = if object.ty().is_array() {
                // Access one index of an array
                object.ty().elem_type()
            } else if object.ty().is_slice() {
                // Access one index of a slice
                object.ty().elem_type()
            } else if object.ty().is_struct() {
                // Access the first field of a stuct
                object.ty().struct_def().1[0].1
            } else {
                panic!("Impossible for access {:?} with {ty:?}", object.ty())
            };
            assert!(range == ty);
            Some(BigInt::ZERO)
        };

        if let Some(x) = final_offset {
            self.build_with_const_offset(object, x, guard, ty, true)
        } else {
            Some(object.extract_inner_expr())
        }
    }

    fn build_with_const_offset(
        &mut self,
        object: Expr,
        offset: BigInt,
        guard: Guard,
        ty: Type,
        bound_check: bool,
    ) -> Option<Expr> {
        let index = self._callback_symex.ctx.constant_integer(offset.clone(), Type::isize_type());
        if bound_check {
            self.bound_check(object.clone(), index.clone(), guard);
        }
        let new_object = if object.is_object() { object } else { self._ctx.object(object) };
        Some(self._ctx.index(new_object, index, ty))
    }

    fn build_slice(
        &mut self,
        object: Expr,
        offset: Option<BigInt>,
        l: Option<usize>,
        r: Option<usize>,
    ) -> Option<Expr> {
        let inner_expr = object.extract_inner_expr();
        assert!(object.ty().is_array() || inner_expr.is_slice());
        let (root_object, start, len) = if object.ty().is_array() {
            let start = match offset {
                Some(o) => o,
                None => BigInt::ZERO,
            };
            let len = object.ty().array_size().expect("array must has len");
            (object, start, BigInt::from(len as usize))
        } else {
            let root_object = inner_expr.extract_object();
            let start = inner_expr.extract_slice_start().extract_constant().to_integer();
            let len = inner_expr.extract_slice_len().extract_constant().to_integer();
            (root_object, start, len)
        };

        let end = start.clone() + len.clone();
        let new_start = start.clone()
            + BigInt::from(match l {
                Some(s) => s,
                _ => 0,
            });
        let new_end = match r {
            Some(e) => start.clone() + BigInt::from(e),
            None => end.clone(),
        };
        let new_len = new_end.clone() - new_start.clone();

        if new_start > new_end || new_end > end || new_len < BigInt::ZERO {
            let msg = NString::from(format!(
                "slicing fail: [{:?}, {:?}) must be in {:?}[{:?}, {:?})",
                &new_start, &new_end, root_object, &start, &end
            ));
            self._callback_symex.claim(msg, self._ctx._false().into());
            None
        } else {
            let slice_start = self._ctx.constant_integer(new_start, Type::usize_type());
            let slice_len = self._ctx.constant_integer(new_len, Type::usize_type());
            Some(self._ctx.slice(root_object, slice_start, slice_len))
        }
    }

    fn make_invalid_object(&mut self, ty: Type) -> Expr {
        let l0_symbol = self._callback_symex.exec_state.l0_symbol(NString::INVALID_OBJECT, ty);
        let l1_symbol = self._callback_symex.exec_state.new_symbol(&l0_symbol, Level::Level1);
        self._ctx.object(l1_symbol)
    }

    fn valid_check(&mut self, object: Expr, state: PlaceState, mode: Mode, guard: Guard) {
        assert!(object.is_object());
        let invalid =
            if state.is_unknown() { self._ctx.invalid(object.clone()) } else { self._ctx._true() };
        let msg = match mode {
            Mode::Read | Mode::Slice(..) => {
                format!("dereference failure: {object:?} is dead").into()
            }
            Mode::Dealloc | Mode::Drop => {
                format!("{} failure: {object:?} is dead", format!("{mode:?}").to_lowercase()).into()
            }
        };
        let mut error = guard.clone();
        error.add(invalid);
        self._callback_symex.claim(msg, error.to_expr());
    }

    fn bound_check(&mut self, object: Expr, index: Expr, guard: Guard) {
        assert!(object.is_object());
        let array_ty = object.ty();
        let s = array_ty.array_size();
        if let Some(len) = s {
            let mut out_of_bound = self._ctx.or(
                self._ctx.lt(index.clone(), self._ctx.constant_isize(0)),
                self._ctx.ge(index.clone(), self._ctx.constant_isize(len as isize)),
            );
            self._callback_symex.rename(&mut out_of_bound);
            out_of_bound.simplify();
            let msg = NString::from(format!("dereference failure: index out of array bound"));
            let mut error = guard.clone();
            error.add(out_of_bound);
            self._callback_symex.claim(msg, error.to_expr());
        }
    }

    fn dereference_null(&mut self, pt: Expr, guard: Guard, mode: Mode) {
        assert!(pt.ty().is_any_ptr());
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
        let alloc_array = self._callback_symex.exec_state.ns.lookup_object(NString::ALLOC_SYM);
        let mut not_alloced =
            self._ctx.not(self._ctx.index(alloc_array, pointer_base, Type::bool_type()));
        self._callback_symex.rename(&mut not_alloced);
        let msg = match mode {
            Mode::Read => NString::from("dereference failure: invalid pointer"),
            // TODO: support more smart pointer
            Mode::Drop => format!("drop failure: uninitilized {:?} pointer", pt.ty().name()).into(),
            Mode::Dealloc => NString::from("dealloc failure: invalid pointer"),
            _ => todo!(),
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
        let tmp_object = if object_ty.is_primitive() || object_ty.is_any_ptr() {
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
        let total_offset = tmp_object.compute_offset();
        let msg = format!(
            "{} failure: the offset is {total_offset:?} != 0",
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
