
use crate::expr::constant::Constant;
use crate::expr::ty::Type;
use crate::NString;

pub trait Tuple<Sort, Ast> {
  fn mk_tuple_sort(&self, ty: Type) -> Sort;
}