
use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
  Add,
  Sub,
  Mul,
  Div,
  Eq,
  Ne,
  Ge,
  Gt,
  Le,
  Lt,
  And,
  Or,
}

impl Debug for BinOp {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Add => write!(f, "+"),
      Self::Sub => write!(f, "-"),
      Self::Mul => write!(f, "*"),
      Self::Div => write!(f, "/"),
      Self::Eq => write!(f, "="),
      Self::Ne => write!(f, "!="),
      Self::Ge => write!(f, ">="),
      Self::Gt => write!(f, ">"),
      Self::Le => write!(f, "<="),
      Self::Lt => write!(f, "<"),
      Self::And => write!(f, "&&"),
      Self::Or => write!(f, "||"),
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
  Not,
  Neg,
}

impl Debug for UnOp {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Not => write!(f, "!"),
      Self::Neg => write!(f, "neg"),
    }
  }
}