
use crate::expr::constant::Constant;
use crate::expr::ty::Type;
use crate::program::program::Program;
use crate::NString;

pub trait Array<Sort, Ast> {
  fn mk_array_sort(&mut self, ty: Type) -> Sort;
  fn mk_const_array(&mut self, constant: Constant, ty: Type) -> Ast;
}