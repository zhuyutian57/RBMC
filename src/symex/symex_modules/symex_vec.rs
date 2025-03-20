

use crate::expr::expr::*;
use crate::expr::ty::*;
use super::super::symex::*;

/// This mod defines symbolic execution of api in std::vec
/// In our memory model, `vec` is a special pointer that owns
/// an infinite array.
/// 
/// TODO: think about how to manage its length.

impl<'cfg> Symex<'cfg> {
  pub fn symex_vec_api(
    &mut self,
    fndef: &FunctionDef,
    args: Vec<Expr>,
    dest: Expr,
  ) {
    todo!();
    // let name = NString::from(fndef.0.trimmed_name());
    // if name == NString::from("Vec::<T>::new") {
    //   todo!();
    // } else {
    //   panic!("Not support for {name:?}");
    // }
  }

  // fn symex_vec_new(&mut self, dest: Expr, fndef: &FunctionDef) {
  //   todo!()
  // }

  // fn symex_from_elem(&mut self, dest: Expr, args: Vec<Expr>) {
  //   todo!()
  // }
}