use std::fmt::Debug;

use num_bigint::BigInt;

use super::ty::Type;

pub type StructField = (Constant, Type);

#[derive(Clone)]
pub enum Constant {
  Bool(bool),
  Integer(BigInt),
  Null,
  Array(Box<Constant>, Type),
  Struct(Vec<StructField>),
}

impl Constant {
  pub fn is_bool(&self) -> bool {
    matches!(self, Constant::Bool(..))
  }
  
  pub fn is_integer(&self) -> bool {
    matches!(self, Constant::Integer(..))
  }
  
  pub fn is_null(&self) -> bool {
    matches!(self, Constant::Null)
  }
  
  pub fn is_array(&self) -> bool {
    matches!(self, Constant::Array(..))
  }
  
  pub fn is_struct(&self) -> bool {
    matches!(self, Constant::Struct(..))
  }

  pub fn to_bool(&self) -> bool {
    match self {
      Constant::Bool(b) => *b,
      _ => panic!("Not bool constant"),
    }
  }

  pub fn to_integer(&self) -> BigInt {
    match self {
      Constant::Integer(i) => i.clone(),
      _ => panic!("Not integer constant"),
    }
  }
}

impl Debug for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::Bool(b)
        => write!(f, "{b}"),
      Constant::Integer(i)
        => write!(f, "{i:?}"),
      Constant::Null
        => write!(f, "null"),
      Constant::Array(v, _)
        => write!(f, "as-const {:?}", *v),
      Constant::Struct(v)
        => write!(f, "{v:?}"),
    }
  }
}