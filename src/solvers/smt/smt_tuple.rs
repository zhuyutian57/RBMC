
use crate::expr::constant::Constant;
use crate::expr::ty::Type;
use crate::program::program::Program;
use crate::NString;

pub trait Tuple<Sort, Ast> {
  fn set_tuples(&mut self);
  fn set_tuples_from_program(&mut self, program: &Program);
  fn create_tuple_sort(&self, ty: Type) -> Sort;
  fn create_tuple(&self, fields: Vec<Ast>, sort: &Sort) -> Ast;
}