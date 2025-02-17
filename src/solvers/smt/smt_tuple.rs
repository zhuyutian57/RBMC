
use crate::expr::ty::Type;
use crate::NString;

pub trait Tuple<T> {
  fn create_tuple(&self, ty: Type) -> T;
}