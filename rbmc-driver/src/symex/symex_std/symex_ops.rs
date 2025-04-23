use stable_mir::CrateDef;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::Guard;
use crate::expr::ty::*;
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::*;
use crate::symex::projection::Mode;

/// This mod defines symbolic execution of api in std::ptr

impl<'cfg> Symex<'cfg> {
    // pub fn symex_ops_api(&mut self, fndef: &FunctionDef, args: Vec<Expr>, dest: Expr) {
    //     let name = NString::from(fndef.0.trimmed_name());
    //     if name == "Index::index" || name == "IndexMut::index_mut" {
    //         self.symex_ops_index(dest, args);
    //     } else {
    //         panic!("Not support for {name:?}");
    //     }
    // }

    // fn symex_ops_index(&mut self, dest: Expr, args: Vec<Expr>) {
    //     let lhs = dest.clone();

    //     let guard = Guard::from(self.ctx._true());
    //     let ty = args[0].ty();

    //     if lhs.ty().is_slice_ptr() {
    //         let pt = args[0].clone();
    //         let (l, r) = self.make_range(args[1].clone());

    //         // Maybe a bug for stable mir: the operands for Range do not
    //         // follow the endian.
    //         let slice = self.make_deref(pt.clone(), Mode::Slice(l, r), self.ctx._true().into(), ty);

    //         let rhs = self.ctx.address_of(self.ctx.object(slice), lhs.ty());
    //         self.assign(lhs.clone(), rhs, guard.clone());
    //         // self.symex_move(pt);
    //         return;
    //     }

    //     if ty.pointee_ty().is_vec() {
    //         let _vec = self.make_deref(args[0].clone(), Mode::Read, guard.clone(), ty.pointee_ty());

    //         // Bound check
    //         let vec_len = self.ctx.vec_len(_vec.clone());
    //         let i = args[1].clone();
    //         let out_of_bound = self.ctx.or(
    //             self.ctx.lt(i.clone(), self.ctx.constant_usize(0)),
    //             self.ctx.ge(i.clone(), vec_len),
    //         );
    //         let msg = NString::from("dereference fail: out of Vec bound");
    //         self.claim(msg, out_of_bound);

    //         // Index vec
    //         let array_ty = _vec.ty().pointee_ty();
    //         let array = self.make_deref(_vec, Mode::Read, guard.clone(), array_ty);
    //         let array_object = self.ctx.object(array);
    //         let elem_ty = array_ty.elem_type();
    //         let index = self.ctx.index(array_object, args[1].clone(), elem_ty);
    //         let index_object = self.ctx.object(index);
    //         let rhs = self.ctx.address_of(index_object, lhs.ty());
    //         self.assign(lhs.clone(), rhs, guard.clone());
    //         return;
    //     }

    //     panic!("Do not support index({:?})", ty.pointee_ty());
    // }

    // pub(super) fn make_range(&mut self, range: Expr) -> (Option<usize>, Option<usize>) {
    //     assert!(range.ty().is_struct());
    //     let name = range.ty().name();
    //     if name == "RangeFull" {
    //         (None, None)
    //     } else {
    //         let fields = range.extract_constant().to_adt().0;
    //         if name == "Range" {
    //             let l = bigint_to_usize(&fields[0].to_integer());
    //             let r = bigint_to_usize(&fields[1].to_integer());
    //             (Some(l), Some(r))
    //         } else if name == "RangeFrom" {
    //             let l = bigint_to_usize(&fields[0].to_integer());
    //             (Some(l), None)
    //         } else if name == "RangeTo" {
    //             let r = bigint_to_usize(&fields[0].to_integer());
    //             (None, Some(r))
    //         } else {
    //             panic!("No support {name:?}")
    //         }
    //     }
    // }
}
