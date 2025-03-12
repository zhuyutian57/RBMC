
use stable_mir::CrateDef;
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use super::super::symex::*;

/// This mod defines symbolic execution of api in std::ptr

impl<'cfg> Symex<'cfg> {
  pub fn symex_ptr_api(
    &mut self,
    fndef: &FunctionDef,
    args: &Vec<Operand>,
    dest: &Place,
  ) {
    let name = NString::from(fndef.0.trimmed_name());
    if name == NString::from("eq") {
      self.symex_ptr_eq(dest, args);
    } else if name == NString::from("null_mut") ||
      name == NString::from("null") {
      self.symex_ptr_null(dest);
    } else {
      panic!("Not support for {name:?}");
    }
  }

  fn symex_ptr_eq(&mut self, dest: &Place, args: &Vec<Operand>) {
    assert!(args.len() == 2);
    let lhs = self.make_project(dest);
    
    let p1 = self.make_operand(&args[0]);
    let p2 = self.make_operand(&args[1]);
    let mut rhs = self.ctx.eq(p1, p2);
    self.replace_predicates(&mut rhs);

    self.assign(lhs, rhs, self.ctx._true());
  }

  fn symex_ptr_null(&mut self, dest: &Place) {
    let lhs = self.make_project(dest);
    let rhs = self.ctx.null(lhs.ty());
    self.assign(lhs, rhs, self.ctx._true());
  }
}