
use crate::expr::ty::Type;
use crate::NString;

pub trait Tuple<Sort, Ast> {
  fn mk_tuple_sort(&self, ty: Type) -> Sort;
  fn mk_tuple_var(&self, name: NString, ty: Type) -> Ast;
}