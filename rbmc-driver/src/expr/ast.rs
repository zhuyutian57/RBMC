use std::fmt::Debug;
use std::hash::Hash;
use std::vec;

use super::constant::*;
use super::op::*;
use super::ty::*;
use crate::symbol::nstring::*;
use crate::symbol::symbol::*;

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
            Terminal::Constant(c) => NString::from(format!("{c:?}")),
            Terminal::Type(t) => NString::from(format!("Type({t:?})")),
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
        }
        .expect("Not constant")
    }

    pub fn to_type(&self) -> Type {
        match self {
            Terminal::Type(t) => Some(t.clone()),
            _ => None,
        }
        .expect("Not constant")
    }

    pub fn to_symbol(&self) -> Symbol {
        match self {
            Terminal::Symbol(s) => Some(s.clone()),
            _ => None,
        }
        .expect("Not constant")
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

    /// `IndexSized(object, index)`: index an array, slice, tuple or struct.
    Index(NodeId, NodeId),
    /// `Store(object, index, value)` updates an array/slice or
    /// a field of a struct.
    Store(NodeId, NodeId, NodeId),

    /// `Pointer(base, offset, meta)`: pointer uniform.
    /// Rust pointer may contains meatadata. For example, slice's metadata is its `len`.
    Pointer(NodeId, NodeId, NodeId),
    /// `PointerBase(pt)` retrieve the ident of a pointer
    PointerBase(NodeId),
    /// `PointerOffset(pt)` retrieve the offset of a pointer
    PointerOffset(NodeId),
    /// `PtrMetaData(pt)` retrieve pointer meta data, such as slice len
    PointerMeta(NodeId),
    /// `Vec(*[T], len, cap)` encodes Vec, three-field tuple,
    /// array pointer, length and capacity
    Vec(NodeId, NodeId, NodeId),
    /// `VecLen(vec)` retrieves the length of a `Vec`
    VecLen(NodeId),
    /// `VecCap(vec)` retrieves the capacity of a `Vec`
    VecCap(NodeId),
    /// Retrieving inner pointer of smart pointer, including `Box`, `Vec`, and son on.
    InnerPointer(NodeId),

    // enum
    /// `Variant(i, x)`: variant `i` with data `x`.
    /// The variant without data is a constant Adt
    Variant(NodeId, NodeId),
    /// `AsVariant(x, i)`: enum `x` as variant
    AsVariant(NodeId, NodeId),
    /// `IsVariant(x, i)`: match `x` with variant `i`
    MatchVariant(NodeId, NodeId),

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

    pub fn is_pointer(&self) -> bool {
        matches!(self, NodeKind::Pointer(..))
    }

    pub fn is_pointer_base(&self) -> bool {
        matches!(self, NodeKind::PointerBase(..))
    }

    pub fn is_pointer_offset(&self) -> bool {
        matches!(self, NodeKind::PointerOffset(..))
    }

    pub fn is_pointer_meta(&self) -> bool {
        matches!(self, NodeKind::PointerMeta(..))
    }

    pub fn is_vec(&self) -> bool {
        matches!(self, NodeKind::Vec(..))
    }

    pub fn is_vec_len(&self) -> bool {
        matches!(self, NodeKind::VecLen(..))
    }

    pub fn is_vec_cap(&self) -> bool {
        matches!(self, NodeKind::VecCap(..))
    }

    pub fn is_inner_pointer(&self) -> bool {
        matches!(self, NodeKind::InnerPointer(..))
    }

    pub fn is_variant(&self) -> bool {
        matches!(self, NodeKind::Variant(..))
    }

    pub fn is_as_variant(&self) -> bool {
        matches!(self, NodeKind::AsVariant(..))
    }

    pub fn is_match_variant(&self) -> bool {
        matches!(self, NodeKind::MatchVariant(..))
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
    pub fn new(kind: NodeKind, ty: Type) -> Self {
        Node { kind, ty }
    }

    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn ty(&self) -> Type {
        self.ty
    }

    /// Retrieve sub-nodes from AST
    pub fn sub_nodes(&self) -> Option<Vec<NodeId>> {
        match &self.kind {
            NodeKind::AddressOf(p) => Some(vec![*p]),
            NodeKind::Aggregate(nodes) => Some(nodes.clone()),
            NodeKind::Binary(_, l, r)
            | NodeKind::Cast(l, r)
            | NodeKind::SameObject(l, r) => Some(vec![*l, *r]),
            NodeKind::Unary(_, o) | NodeKind::Object(o) => Some(vec![*o]),
            NodeKind::Slice(o, s, l) => Some(vec![*o, *s, *l]),
            NodeKind::Ite(c, tv, fv) => Some(vec![*c, *tv, *fv]),
            NodeKind::Index(o, i) => Some(vec![*o, *i]),
            NodeKind::Store(o, i, v) => Some(vec![*o, *i, *v]),
            NodeKind::Pointer(a, o, m) => Some(vec![*a, *o, *m]),
            NodeKind::PointerBase(p)
            | NodeKind::PointerOffset(p)
            | NodeKind::PointerMeta(p)
            | NodeKind::VecLen(p)
            | NodeKind::VecCap(p)
            | NodeKind::InnerPointer(p) => Some(vec![*p]),
            NodeKind::Vec(p, l, c) => Some(vec![*p, *l, *c]),
            NodeKind::Variant(i, x) => Some(vec![*i, *x]),
            NodeKind::AsVariant(x, i) | NodeKind::MatchVariant(x, i) => Some(vec![*x, *i]),
            NodeKind::Move(o) | NodeKind::Valid(o) | NodeKind::Invalid(o) => Some(vec![*o]),
            NodeKind::NullObject | NodeKind::Unknown(_) => Some(vec![]),
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
