
use crate::expr::ty::Type;
use crate::NString;

pub trait Array<Sort, Ast> {
  fn mk_array_sort(&self, ty: Type) -> Sort;
  fn mk_array_var(&self, name: NString, ty: Type) -> Ast;
}