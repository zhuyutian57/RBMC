
use std::fmt::Debug;
use std::hash::Hash;

use crate::symbol::{symbol::*, nstring::*};

use super::constant::*;
use super::ty::*;

pub type TerminalId = usize;

#[derive(Clone)]
pub enum Terminal {
  Constant(Constant),
  Symbol(Symbol),
  Layout(Type),
}

impl Terminal {
  pub fn identifier(&self) -> NString {
    match self {
      Terminal::Constant(c) =>
        NString::from(format!("{c:?}")),
      Terminal::Symbol(s) => s.name(),
      Terminal::Layout(t) =>
        NString::from(format!("Layout({t:?})")),
    }
  }
}

impl Debug for Terminal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Terminal::Constant(c) => write!(f, "{c:?}"),
      Terminal::Symbol(s) => write!(f, "{s:?}"),
      Terminal::Layout(t) => write!(f, "Layout({t:?})"),
    }
  }
}

pub type NodeId = usize;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
  Binary(BinOp, NodeId, NodeId),
  Unary(UnOp, NodeId),

  /// Terminal is the bridge connecting ast and terminals
  Terminal(TerminalId),
  
  Object(NodeId),
}

impl NodeKind {
  pub fn is_binary(&self) -> bool {
    matches!(self, NodeKind::Binary(_, _, _))
  }

  pub fn is_unary(&self) -> bool {
    matches!(self, NodeKind::Unary(_, _))
  } 

  pub fn is_terminal(&self) -> bool {
    matches!(self, NodeKind::Terminal(_))
  } 

  pub fn is_object(&self) -> bool {
    matches!(self, NodeKind::Object(_))
  } 
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
  kind: NodeKind,
  ty: Type,
}

impl Node {
  pub fn binary(
    op: BinOp,
    lhs: NodeId,
    rhs: NodeId,
    ty: Type,
  ) -> Self {
    Node {
      kind: NodeKind::Binary(op, lhs, rhs),
      ty
    }
  }

  pub fn unary(
    op: UnOp,
    operand: NodeId,
    ty: Type
  ) -> Self {
    Node {
      kind: NodeKind::Unary(op, operand),
      ty
    }
  }

  pub fn terminal(
    i: TerminalId,
    ty: Type
  ) -> Self {
    Node { kind: NodeKind::Terminal(i), ty }
  }

  pub fn object(
    i: NodeId,
    ty: Type
  ) -> Self {
    Node { kind: NodeKind::Object(i), ty }
  }

  pub fn kind(&self) -> NodeKind { self.kind }

  pub fn ty(&self) -> Type { self.ty }

  pub fn terminal_id(&self) -> Option<TerminalId> {
    match self.kind {
      NodeKind::Terminal(t) => Some(t),
      _ => None,
    }
  }
}

impl Hash for Node {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
      self.kind.hash(state);
      if matches!(self.kind, NodeKind::Terminal(_)) {
        self.ty.hash(state);
      }
  }
}