
use stable_mir::ty::*;
use stable_mir::CrateDef;
use stable_mir::mir::*;
use stable_mir::CrateDefType;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::*;
use crate::symex::projection::Mode;
use super::super::symex::*;

/// This mod defines symbolic execution of api in std::ptr

impl<'cfg> Symex<'cfg> {
  pub fn symex_ops_api(
    &mut self,
    fndef: &FunctionDef,
    args: &Vec<Operand>,
    dest: &Place,
  ) {
    let name = NString::from(fndef.0.trimmed_name());
    if name == NString::from("Index::index") ||
       name == NString::from("IndexMut::index_mut") {
      self.symex_index(dest, args);
    } else {
      panic!("Not support for {name:?}");
    }
  }

  fn symex_index(&mut self, dest: &Place, args: &Vec<Operand>) {
    let lhs = self.make_project(dest);
    let ty = lhs.ty();
    assert!(ty.is_ref());
    
    if ty.is_slice_ref() {
      let pt = self.make_operand(&args[0]);
      let idx = self.make_operand(&args[1]);
      assert!(idx.ty().struct_name() == NString::from("Range"));
      let fields = idx.extract_struct_fields();
      assert!(fields.len() == 2);

      // Maybe a bug for stable mir: the operands for Range do not
      // follow the endian.
      let l = bigint_to_usize(&fields[1].0.to_integer());
      let r = bigint_to_usize(&fields[0].0.to_integer());
      let slice =
        self.make_deref(pt.clone(), Mode::Slice(l, r), self.ctx._true());
      
      // Build success
      if let Some(s) = slice {
        let rhs = self.ctx.address_of(self.ctx.object(s), ty);
        self.assign(lhs, rhs, self.ctx._true());
        self.symex_move(pt);
      }
      return;
    }

    panic!("Do not support index({ty:?})");
  }
}