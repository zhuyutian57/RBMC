
use crate::expr::constant::Constant;
use crate::expr::ty::Type;
use crate::NString;

pub trait Tuple<Sort, Ast> {
  fn create_tuple_sort(&self, ty: Type) -> Sort;
  fn create_tuple(&self, fields: Vec<Ast>, sort: &Sort) -> Ast;
}