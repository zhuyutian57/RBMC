use std::fmt::Debug;

use num_bigint::BigInt;

use super::ty::Type;

pub type ConstantField = (Constant, Type);

#[derive(Clone)]
pub enum Constant {
    Bool(bool),
    Integer(BigInt),
    Null(Type),
    Array(Box<Constant>, Type),
    Struct(Vec<ConstantField>, Type),
}

impl Constant {
    pub fn is_bool(&self) -> bool {
        matches!(self, Constant::Bool(..))
    }

    pub fn is_integer(&self) -> bool {
        matches!(self, Constant::Integer(..))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Constant::Null(..))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Constant::Array(..))
    }

    pub fn is_struct(&self) -> bool {
        matches!(self, Constant::Struct(..))
    }

    pub fn to_bool(&self) -> bool {
        match self {
            Constant::Bool(b) => *b,
            _ => panic!("Not constant bool"),
        }
    }

    pub fn to_integer(&self) -> BigInt {
        match self {
            Constant::Integer(i) => i.clone(),
            _ => panic!("Not constant integer"),
        }
    }

    pub fn to_struct_fields(&self) -> Vec<ConstantField> {
        match self {
            Constant::Struct(fields, _) => fields.clone(),
            _ => panic!("Not constant struct"),
        }
    }
}

impl Debug for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constant::Bool(b) => write!(f, "{b}"),
            Constant::Integer(i) => write!(f, "{i:?}"),
            Constant::Null(..) => write!(f, "null"),
            Constant::Array(v, _) => write!(f, "as-const {:?}", *v),
            Constant::Struct(v, ty) => write!(f, "{ty:?} {v:?}"),
        }
    }
}
