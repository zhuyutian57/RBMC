
use std::fmt::Debug;
use std::hash::Hash;
use std::vec;

use crate::symbol::symbol::*;
use crate::symbol::nstring::*;
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

  pub fn is_constant(&self) -> bool {
    matches!(self, Terminal::Constant(..))
  }
  
  pub fn is_type(&self) -> bool {
    matches!(self, Terminal::Type(..))
  }
  
  pub fn is_symbol(&self) -> bool {
    matches!(self, Terminal::Symbol(..))
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum NodeKind {
  /// Terminal is the bridge connecting with terminals.
  Terminal(TerminalId),
  /// Get address from a place. Create `Ref` if necessary.
  AddressOf(NodeId),
  /// Aggregate value such as tuple and struct
  Aggregate(Vec<NodeId>),
  /// Binary expression
  Binary(BinOp, NodeId, NodeId),
  /// Unary expression
  Unary(UnOp, NodeId),
  /// If cond { true_expresion } else { false_expression }
  Ite(NodeId, NodeId, NodeId),
  /// Type casting
  Cast(NodeId, NodeId),

  /// Unified wrapper for objects, including array, slice,
  /// struct, tuple, and so on. Moreover, heap objects and
  /// stack objects are included.
  Object(NodeId),
  /// `Slice(object, start, len)` represent a slice
  Slice(NodeId, NodeId, NodeId),
  /// A pointer's value is the address of an object...
  SameObject(NodeId, NodeId),

  /// `IndexOf(object, index)` represents a load from an array/slice
  /// or a load of a field of a struct.
  Index(NodeId, NodeId),
  /// `Store(object, index, value)` updates an array/slice or
  /// a field of a struct.
  Store(NodeId, NodeId, NodeId),

  /// `Box(*const T)` encodes Box pointer,a one-field tuple 
  Box(NodeId),
  /// `PointerIdent(pt)` retrieve the ident of a pointer
  PointerIdent(NodeId),
  /// `PointerOffset(pt)` retrieve the offset of a pointer
  PointerOffset(NodeId),
  /// `PtrMetaData(pt)` retrieve pointer meta data, such as slice len
  PointerMeta(NodeId),

  // Predicates for symbolic execution. Before generating VCC,
  // all predicates must be replaced to some expression.

  /// `Move(expr)`: move a value
  Move(NodeId),
  /// `Valid(object)`: `object` is alloced
  Valid(NodeId),
  /// `Invalid(object)`: `object` is not alloced
  Invalid(NodeId),
  /// Representing dereference of null
  NullObject,
  /// `Unknown(type)` an unknown object with type
  Unknown(Type),
}

impl NodeKind {
  pub fn is_terminal(&self) -> bool {
    matches!(self, NodeKind::Terminal(_))
  }

  pub fn is_aggregate(&self) -> bool {
    matches!(self, NodeKind::Aggregate(_))
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
  
  pub fn is_ite(&self) -> bool {
    matches!(self, NodeKind::Ite(..))
  }

  pub fn is_cast(&self) -> bool {
    matches!(self, NodeKind::Cast(..))
  }

  pub fn is_object(&self) -> bool {
    matches!(self, NodeKind::Object(..))
  }
  
  pub fn is_slice(&self) -> bool {
    matches!(self, NodeKind::Slice(..))
  }

  pub fn is_same_object(&self) -> bool {
    matches!(self, NodeKind::SameObject(..))
  }

  pub fn is_index(&self) -> bool {
    matches!(self, NodeKind::Index(..))
  }

  pub fn is_store(&self) -> bool {
    matches!(self, NodeKind::Store(..))
  }

  pub fn is_box(&self) -> bool {
    matches!(self, NodeKind::Box(..))
  }

  pub fn is_pointer_ident(&self) -> bool {
    matches!(self, NodeKind::PointerIdent(..))
  }

  pub fn is_pointer_offset(&self) -> bool {
    matches!(self, NodeKind::PointerOffset(..))
  }

  pub fn is_pointer_meta(&self) -> bool {
    matches!(self, NodeKind::PointerMeta(..))
  }

  pub fn is_move(&self) -> bool {
    matches!(self, NodeKind::Move(..))
  }

  pub fn is_valid(&self) -> bool {
    matches!(self, NodeKind::Valid(..))
  }

  pub fn is_invalid(&self) -> bool {
    matches!(self, NodeKind::Invalid(..))
  }

  pub fn is_null_object(&self) -> bool {
    matches!(self, NodeKind::NullObject)
  }

  pub fn is_unknown(&self) -> bool {
    matches!(self, NodeKind::Unknown(..))
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Node {
  kind: NodeKind,
  ty: Type,
}

impl Node {
  pub fn new(kind: NodeKind, ty: Type) -> Self { Node { kind, ty } }

  pub fn kind(&self) -> &NodeKind { &self.kind }

  pub fn ty(&self) -> Type { self.ty }

  /// Retrieve sub-nodes from AST
  pub fn sub_nodes(&self) -> Option<Vec<NodeId>> {
    match &self.kind {
      NodeKind::AddressOf(p)
        => Some(vec![*p]),
      NodeKind::Aggregate(nodes)
        => Some(nodes.clone()),
      NodeKind::Binary(_, l, r) |
      NodeKind::Cast(l, r) |
      NodeKind::SameObject(l, r)
        => Some(vec![*l, *r]),
      NodeKind::Unary(_, o) |
      NodeKind::Object(o)
        => Some(vec![*o]),
      NodeKind::Slice(o, s, l)
        => Some(vec![*o, *s, *l]),
      NodeKind::Ite(c, tv, fv)
        => Some(vec![*c, *tv, *fv]),
      NodeKind::Index(o, i)
        => Some(vec![*o, *i]),
      NodeKind::Store(o, i, v)
        => Some(vec![*o, *i, *v]),
      NodeKind::Box(p) |
      NodeKind::PointerIdent(p) |
      NodeKind::PointerMeta(p)
        => Some(vec![*p]),
      NodeKind::Move(o) |
      NodeKind::Valid(o) |
      NodeKind::Invalid(o)
        => Some(vec![*o]),
      NodeKind::NullObject | NodeKind::Unknown(_)
        => Some(vec![]),
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