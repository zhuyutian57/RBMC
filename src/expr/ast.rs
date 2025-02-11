
use std::fmt::Debug;
use std::hash::Hash;
use std::vec;

use crate::symbol::{symbol::*, nstring::*};
use super::constant::*;
use super::op::*;
use super::predicates::*;
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

  pub fn to_constant(&self) -> Constant {
    match self {
      Terminal::Constant(c) => Some(c.clone()),
      _ => None,
    }.expect("Not constant")
  }

  pub fn to_type(&self) -> Type {
    match self {
      Terminal::Type(t) => Some(t.clone()),
      _ => None,
    }.expect("Not constant")
  }

  pub fn to_symbol(&self) -> Symbol {
    match self {
      Terminal::Symbol(s) => Some(s.clone()),
      _ => None,
    }.expect("Not constant")
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
  /// Terminal is the bridge connecting with terminals.
  Terminal(TerminalId),
  /// Get address from a place. Create `Ref` if necessary.
  AddressOf(NodeId),
  Binary(BinOp, NodeId, NodeId),
  Unary(UnOp, NodeId),
  Cast(NodeId, NodeId),
  /// Unified form for object, including stack objects and heap objects.
  Object(Ownership, NodeId),
  /// `IndexOf` represents a visit for a struct or array in an unified form.
  IndexOf(NodeId, NodeId),
  Ite(NodeId, NodeId, NodeId),
  /// A pointer's value is the address of an object...
  SameObject(NodeId, NodeId),
  /// `With(obj, i, value)` updates the value of obj
  With(NodeId, NodeId, NodeId),
}

impl NodeKind {
  pub fn is_terminal(&self) -> bool {
    matches!(self, NodeKind::Terminal(_))
  }

  pub fn is_address_of(&self) -> bool {
    matches!(self, NodeKind::AddressOf(_))
  }

  pub fn is_binary(&self) -> bool {
    matches!(self, NodeKind::Binary(..))
  }

  pub fn is_unary(&self) -> bool {
    matches!(self, NodeKind::Unary(..))
  }

  pub fn is_cast(&self) -> bool {
    matches!(self, NodeKind::Cast(..))
  }

  pub fn is_object(&self) -> bool {
    matches!(self, NodeKind::Object(..))
  }

  pub fn is_index_of(&self) -> bool {
    matches!(self, NodeKind::IndexOf(..))
  }

  pub fn is_ite(&self) -> bool {
    matches!(self, NodeKind::Ite(..))
  }

  pub fn is_same_object(&self) -> bool {
    matches!(self, NodeKind::SameObject(..))
  }

  pub fn is_with(&self) -> bool {
    matches!(self, NodeKind::With(..))
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
      NodeKind::Object(_, o)
        => Some(vec![o]),
      NodeKind::IndexOf(o, i)
        => Some(vec![o, i]),
      NodeKind::Ite(c, tv, fv)
        => Some(vec![c, tv, fv]),
      NodeKind::With(o, i, v)
        => Some(vec![o, i, v]),
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