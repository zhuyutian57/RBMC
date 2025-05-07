use std::fmt::Debug;

use num_bigint::BigInt;

use crate::program::program::bigint_to_usize;

use super::ty::Type;

#[derive(Clone)]
pub enum Constant {
    Bool(bool),
    Integer(BigInt),
    Null(Type),
    Array(Box<Constant>, Type),
    /// Constant `struct/tuple/enum`. The data for each `struct/tuple` is stored in order.
    /// For `enum`, the first value is variant index.
    Adt(Vec<Constant>, Type),
    /// Zero-sized type is a constant
    Zst(Type),
}

impl Constant {
    pub fn is_integer(&self) -> bool {
        matches!(self, Constant::Integer(..))
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Constant::Null(..))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Constant::Array(..))
    }

    pub fn is_adt(&self) -> bool {
        matches!(self, Constant::Adt(..))
    }

    pub fn is_zst(&self) -> bool {
        matches!(self, Constant::Zst(..))
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

    pub fn to_array(&self) -> (Constant, Type) {
        match self {
            Constant::Array(c, t) => ((**c).clone(), *t),
            _ => panic!("Not constant array"),
        }
    }

    pub fn to_adt(&self) -> (Vec<Constant>, Type) {
        match self {
            Constant::Adt(fields, ty) => (fields.clone(), *ty),
            _ => panic!("Not constant adt"),
        }
    }

    pub fn to_zst(&self) -> Type {
        match self {
            Constant::Zst(ty) => *ty,
            _ => panic!("Not constant ZST"),
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
            Constant::Adt(v, ty) => {
                if !ty.is_enum() {
                    write!(f, "{:?} {v:?}", ty.name())
                } else {
                    let i = bigint_to_usize(&v[0].to_integer());
                    let variant_name = ty.enum_def().1[i].0;
                    if v.len() == 1 {
                        write!(f, "{variant_name:?}")
                    } else {
                        write!(f, "{variant_name:?}({:?})", v[1])
                    }
                }
            }
            Constant::Zst(ty) => write!(f, "ZST({ty:?})"),
        }
    }
}
