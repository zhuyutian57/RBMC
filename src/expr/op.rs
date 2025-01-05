
use std::fmt::{Debug, Error};

use stable_mir::mir;

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

impl From<mir::BinOp> for BinOp {
  fn from(value: mir::BinOp) -> Self {
    match value {
      mir::BinOp::Add => Ok(BinOp::Add),
      mir::BinOp::Sub => Ok(BinOp::Sub),
      mir::BinOp::Mul => Ok(BinOp::Mul),
      mir::BinOp::Div => Ok(BinOp::Div),
      mir::BinOp::Eq => Ok(BinOp::Eq),
      mir::BinOp::Ne => Ok(BinOp::Ne),
      mir::BinOp::Le => Ok(BinOp::Le),
      mir::BinOp::Lt => Ok(BinOp::Lt),
      mir::BinOp::Ge => Ok(BinOp::Ge),
      mir::BinOp::Gt => Ok(BinOp::Gt),
      mir::BinOp::BitAnd => Ok(BinOp::And),
      mir::BinOp::BitOr => Ok(BinOp::Or),
      _ => Err(Error),
    }.expect("Do not support")
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

impl From<mir::UnOp> for UnOp {
  fn from(value: mir::UnOp) -> Self {
    match value {
      mir::UnOp::Not => Ok(UnOp::Not),
      mir::UnOp::Neg => Ok(UnOp::Neg),
      _ => Err(Error),
    }.expect("Do not support")
  }
}