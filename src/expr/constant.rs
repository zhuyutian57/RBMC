use std::fmt::Debug;

pub type Sign = bool;

#[derive(Clone)]
pub enum Constant {
  Bool(bool),
  Integer(Sign, u128),
  Struct(Vec<Constant>),
}

impl Constant {
  pub fn bool_value(&self) -> bool {
    match self {
      Constant::Bool(b) => *b,
      _ => panic!("Not bool constant"),
    }
  }

  pub fn integer_value(&self) -> (Sign, u128) {
    match self {
      Constant::Integer(s, u) => (*s, *u),
      _ => panic!("Not integer constant"),
    }
  }

  pub fn fields(&self) -> Vec<Constant> {
    match self {
      Constant::Struct(f) => f.clone(),
      _ => panic!("Not struct constant"),
    }
  }
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