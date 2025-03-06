
use num_bigint::BigInt;

use crate::expr::expr::Expr;
use crate::expr::ty::Type;

pub trait Tuple<Sort, Ast> {
  fn create_tuple_sort(&mut self, ty: Type) -> Sort;
  fn create_tuple(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
  fn mk_tuple_select(&mut self, object: Expr, field: BigInt) -> Ast;
  fn mk_tuple_store(&mut self, object: Expr, field: BigInt, value: Expr) -> Ast;
}