
use crate::expr::constant::Constant;
use crate::expr::ty::Type;
use crate::program::program::Program;
use crate::NString;

pub trait Tuple<Sort, Ast> {
  fn mk_tuple_sort(&mut self, ty: Type) -> Sort;
  fn mk_tuple(&mut self, fields: Vec<Ast>, ty: Type) -> Ast;
}