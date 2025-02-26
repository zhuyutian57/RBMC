use std::fmt::Debug;

use super::ty::Type;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Hash)]
pub struct BigInt(pub bool, pub u128);

impl BigInt {
  pub fn zero() -> Self { BigInt(false, 0) }

  pub fn is_negative(&self) -> bool { self.0 }
  pub fn is_positive(&self) -> bool { !self.0 }
  
  pub fn to_int(&self) -> i128 {
    (self.1 as i128) * if self.0 { -1 } else { 1 }
  }

  pub fn to_uint(&self) -> u128 {
    assert!(self.is_positive());
    self.1
  }
}

impl Debug for BigInt {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}{}", if self.0 { "-" } else { "" }, self.1)
  }
}

impl ToString for BigInt {
  fn to_string(&self) -> String { format!("{self:?}") }
}

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
      Constant::Integer(i) => *i,
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