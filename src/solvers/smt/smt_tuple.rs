
use crate::expr::constant::Constant;
use crate::expr::expr::Expr;
use crate::expr::ty::Type;
use crate::program::program::Program;
use crate::NString;

pub trait Tuple<Sort, Ast> {
  fn create_tuple_sort(&mut self, ty: Type) -> Sort;
  fn create_tuple(&mut self, fields: Vec<Ast>, ty: Type) -> Ast;
  fn load_tuple_field(&mut self, object: Expr, field: usize) -> Ast;
}