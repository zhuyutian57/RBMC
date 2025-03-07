
use stable_mir::mir::*;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use super::super::symex::*;

/// This mod defines symbolic execution of api in std::vec

impl<'cfg> Symex<'cfg> {
  pub fn symex_vec_api(
    &mut self,
    fndef: &FunctionDef,
    args: &Vec<Operand>,
    dest: &Place,
  ) {
    let name = NString::from(fndef.0.trimmed_name());
    if name == NString::from("Vec::<T>::new") {
      todo!();
    } else {
      panic!("Not support for {name:?}");
    }
  }

  fn symex_vec_new(&mut self, dest: &Place, fndef: &FunctionDef) {
    let elem_ty = Type::from(fndef.1.0[0].expect_ty());
    let vec_ty = Type::infinite_array_type(elem_ty);
    let vector = self.exec_state.new_object(vec_ty);

    let lhs = self.make_project(dest);
    
  }
}