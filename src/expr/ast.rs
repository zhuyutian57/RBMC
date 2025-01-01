
use std::fmt::Debug;
use std::hash::Hash;
use std::vec;

use crate::symbol::{symbol::*, nstring::*};

use super::constant::*;
use super::op::*;
use super::ty::*;

pub type TerminalId = usize;

#[derive(Clone)]
pub(super) enum Terminal {
  Constant(Constant),
  Layout(Type),
  Symbol(Symbol),
}

impl Terminal {
  pub fn identifier(&self) -> NString {
    match self {
      Terminal::Constant(c) =>
        NString::from(format!("{c:?}")),
      Terminal::Layout(t) =>
        NString::from(format!("Layout({t:?})")),
      Terminal::Symbol(s) => s.name(),
    }
  }
}

impl Debug for Terminal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Terminal::Constant(c) => write!(f, "{c:?}"),
      Terminal::Layout(t) => write!(f, "Layout({t:?})"),
      Terminal::Symbol(s) => write!(f, "{s:?}"),
    }
  }
}

pub type NodeId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum NodeKind {
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
pub(super) struct Node {
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

  /// Retrieve sub-nodes from AST
  pub fn sub_nodes(&self) -> Option<Vec<NodeId>> {
    match self.kind {
      NodeKind::Binary(_, l, r)
        => Some(vec![l, r]),
      NodeKind::Unary(_, o) |
      NodeKind::Object(o)
        => Some(vec![o]),
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