
use std::fmt::Debug;
use std::fmt::Error;

use stable_mir::CrateDef;
use stable_mir::ty::*;

/// A wrapper for `Ty` in MIR
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type(Ty);

impl Type {
  pub fn bool_type() -> Self { Type(Ty::bool_ty()) }

  pub fn is_bool(&self) -> bool { self.0.kind().is_bool() }

  pub fn is_ref(&self) -> bool { self.0.kind().is_ref() }

  pub fn is_raw_ptr(&self) -> bool { self.0.kind().is_raw_ptr() }

  pub fn is_box(&self) -> bool { self.0.kind().is_box() }

  /// `Box` is also a ptr by our semantic
  pub fn is_ptr(&self) -> bool {
    self.is_ref() || self.is_raw_ptr() || self.is_box()
  }
}

impl Debug for Type {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let kind = self.0.kind();
    let rigid = kind.rigid().unwrap();
    match rigid {
      RigidTy::Bool => write!(f, "bool"),
      RigidTy::Int(i) =>
        write!(f, "{}", format!("{i:?}").to_lowercase()),
      RigidTy::Uint(u) =>
        write!(f, "{}", format!("{u:?}").to_lowercase()),
      RigidTy::Adt(def, _) => write!(f, "{}", def.trimmed_name()),
      RigidTy::RawPtr(ty, m) => {
        let t = Type::from(*ty);
        write!(f, "RawPtr({t:?}, {m:?})")
      },
      RigidTy::Ref(_, ty, m) => {
        let t = Type::from(*ty);
        write!(f, "Ref({t:?}, {m:?})")
      }
      RigidTy::Tuple(data) => {
        if data.is_empty() {
          write!(f, "Unit")
        } else {
          Err(Error)
        }
      }
      _ => Err(Error),
    }
  }
}

impl From<Ty> for Type {
  fn from(value: Ty) -> Self {
    assert!(matches!(value.kind(), TyKind::RigidTy(_)));
    Type(value)
  }
}