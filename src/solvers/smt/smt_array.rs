
use crate::expr::ty::Type;
use crate::program::program::Program;
use crate::NString;

pub trait Array<Sort, Ast> {
  fn set_arrays_from_program(&mut self, program: &Program);
}