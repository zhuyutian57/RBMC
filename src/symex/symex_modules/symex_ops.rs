
use stable_mir::CrateDef;
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
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
    if name == NString::from("IndexMut::index_mut") {
      self.symex_index_mut(dest, args);
    } else {
      panic!("Not support for {name:?}");
    }
  }

  fn symex_index_mut(&mut self, dest: &Place, args: &Vec<Operand>) {
    println!("{args:?}");
    todo!()
  }
}