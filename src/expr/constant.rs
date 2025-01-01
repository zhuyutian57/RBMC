use std::{fmt::Debug, hash::Hash, rc::Rc};

pub type Sign = bool;

#[derive(Clone)]
pub enum Constant {
  Bool(bool),
  Integer(Sign, u128),
  Struct(Vec<Constant>),
}

impl Debug for Constant {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Constant::Bool(b) =>
        f.write_fmt(format_args!("{b}")),
      Constant::Integer(s, v) =>
        f.write_fmt(format_args!("{}{v}",if *s { "-" } else { "" })),
      Constant::Struct(v) =>
        write!(f, "{v:?}"),
    }
  }
}