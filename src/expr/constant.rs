use std::fmt::Debug;

use super::ty::Type;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Hash)]
pub struct BigInt(pub bool, pub u128);

impl BigInt {
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
  Array(Box<Constant>, Type),
  Struct(Vec<StructField>),
}

impl Constant {
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
      Constant::Bool(b) =>
        f.write_fmt(format_args!("{b}")),
      Constant::Integer(i) =>
        f.write_fmt(format_args!("{i:?}")),
        Constant::Array(v, _) =>
        f.write_fmt(format_args!("as-const {:?}", *v)),
      Constant::Struct(v) =>
        write!(f, "{v:?}"),
    }
  }
}