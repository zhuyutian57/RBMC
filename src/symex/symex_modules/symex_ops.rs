use stable_mir::CrateDef;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::*;
use crate::symex::projection::Mode;

/// This mod defines symbolic execution of api in std::ptr

impl<'cfg> Symex<'cfg> {
    pub fn symex_ops_api(&mut self, fndef: &FunctionDef, args: Vec<Expr>, dest: Expr) {
        let name = NString::from(fndef.0.trimmed_name());
        if name == "Index::index" || name == "IndexMut::index_mut" {
            self.symex_ops_index(dest, args);
        } else {
            panic!("Not support for {name:?}");
        }
    }

    fn symex_ops_index(&mut self, dest: Expr, args: Vec<Expr>) {
        let lhs = dest.clone();
        let ty = lhs.ty();
        assert!(ty.is_ref());

        if ty.is_slice_ptr() {
            let pt = args[0].clone();
            let (l, r) = self.make_range(args[1].clone());

            // Maybe a bug for stable mir: the operands for Range do not
            // follow the endian.
            let slice = self.make_deref(pt.clone(), Mode::Slice(l, r), self.ctx._true().into(), ty);

            // Build success
            if let Some(s) = slice {
                let rhs = self.ctx.address_of(self.ctx.object(s), ty);
                self.assign(lhs, rhs, self.ctx._true().into());
                // self.symex_move(pt);
            }
            return;
        }

        panic!("Do not support index({:?})", args[0]);
    }

    pub(super) fn make_range(&mut self, range: Expr) -> (Option<usize>, Option<usize>) {
        assert!(range.ty().is_struct());
        let name = range.ty().name();
        if name == "RangeFull" {
            (None, None)
        } else {
            let fields = range.extract_constant().to_struct_fields();
            if name == "Range" {
                // Maybe a bug for MIR
                let l = bigint_to_usize(&fields[1].0.to_integer());
                let r = bigint_to_usize(&fields[0].0.to_integer());
                (Some(l), Some(r))
            } else if name == "RangeFrom" {
                let l = bigint_to_usize(&fields[0].0.to_integer());
                (Some(l), None)
            } else if name == "RangeTo" {
                let r = bigint_to_usize(&fields[0].0.to_integer());
                (None, Some(r))
            } else {
                panic!("No support {name:?}")
            }
        }
    }
}
