

use crate::expr::expr::Expr;
use crate::expr::ty::Type;

pub trait Slice<Sort, Ast> {
  fn create_slice_pointer_sort(&mut self, ty: Type) -> Sort;
}