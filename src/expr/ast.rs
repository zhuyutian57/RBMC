
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
  Type(Type),
  Symbol(Symbol),
}

impl Terminal {
  pub fn identifier(&self) -> NString {
    match self {
      Terminal::Constant(c) =>
        NString::from(format!("{c:?}")),
      Terminal::Type(t) =>
        NString::from(format!("Type({t:?})")),
      Terminal::Symbol(s) => s.name(),
    }
  }
}

impl Debug for Terminal {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Terminal::Constant(c) => write!(f, "{c:?}"),
      Terminal::Type(t) => write!(f, "Type({t:?})"),
      Terminal::Symbol(s) => write!(f, "{s:?}"),
    }
  }
}

pub type NodeId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum NodeKind {
  /// Terminal is the bridge connecting ast and terminals
  Terminal(TerminalId),
  /// Get address from a place. Create `Ref` if necessary.
  AddressOf(NodeId),
  Binary(BinOp, NodeId, NodeId),
  Unary(UnOp, NodeId),
  Cast(NodeId, NodeId),
  Object(NodeId),
  /// `IndexOf` represents a visit for a struct or array
  /// in an unified form
  IndexOf(NodeId, NodeId),
  Ite(NodeId, NodeId, NodeId),
  SameObject(NodeId, NodeId),
}

impl NodeKind {
  pub fn is_terminal(&self) -> bool {
    matches!(self, NodeKind::Terminal(_))
  }

  pub fn is_address_of(&self) -> bool {
    matches!(self, NodeKind::AddressOf(_))
  }

  pub fn is_binary(&self) -> bool {
    matches!(self, NodeKind::Binary(_, _, _))
  }

  pub fn is_unary(&self) -> bool {
    matches!(self, NodeKind::Unary(_, _))
  }

  pub fn is_cast(&self) -> bool {
    matches!(self, NodeKind::Cast(_, _))
  }

  pub fn is_object(&self) -> bool {
    matches!(self, NodeKind::Object(_))
  }

  pub fn is_index_of(&self) -> bool {
    matches!(self, NodeKind::IndexOf(_, _))
  }

  pub fn is_ite(&self) -> bool {
    matches!(self, NodeKind::Ite(_, _, _))
  }

  pub fn is_same_object(&self) -> bool {
    matches!(self, NodeKind::SameObject(_, _))
  }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
  kind: NodeKind,
  ty: Type,
}

impl Node {
  pub fn new(kind: NodeKind, ty: Type) -> Self { Node { kind, ty } }

  pub fn kind(&self) -> NodeKind { self.kind }

  pub fn ty(&self) -> Type { self.ty }

  /// Retrieve sub-nodes from AST
  pub fn sub_nodes(&self) -> Option<Vec<NodeId>> {
    match self.kind {
      NodeKind::AddressOf(p)
        => Some(vec![p]),
      NodeKind::Binary(_, l, r) |
      NodeKind::Cast(l, r) |
      NodeKind::SameObject(l, r)
        => Some(vec![l, r]),
      NodeKind::Unary(_, o) |
      NodeKind::Object(o)
        => Some(vec![o]),
      NodeKind::IndexOf(o, i)
        => Some(vec![o, i]),
      NodeKind::Ite(c, tv, fv)
        => Some(vec![c, tv, fv]),
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