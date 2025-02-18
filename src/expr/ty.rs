
use std::fmt::Debug;
use std::fmt::Error;

use stable_mir::CrateDef;
use stable_mir::mir::*;
use stable_mir::ty::*;

use crate::NString;

pub type FieldDef = (NString, Type);
pub type StructDef = (NString, Vec<FieldDef>);

/// A wrapper for `Ty` in MIR
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type(Ty);

impl Type {
  pub fn bool_type() -> Self {
    Type::from(Ty::bool_ty())
  }

  pub fn signed_type(ty: IntTy) -> Self {
    Type::from(Ty::signed_ty(ty))
  }

  pub fn unsigned_type(ty: UintTy) -> Self {
    Type::from(Ty::unsigned_ty(ty))
  }

  pub fn ptr_type(pointee_ty: Ty, mutability: Mutability) -> Self {
    Type::from(Ty::new_ptr(pointee_ty, mutability))
  }

  pub fn ref_type(reg: Region, pointee_ty: Ty, mutability: Mutability) -> Self {
    Type::from(Ty::new_ref(reg, pointee_ty, mutability))
  }

  pub fn is_bool(&self) -> bool { self.0.kind().is_bool() }

  pub fn is_signed(&self) -> bool {
    self.0.kind().is_signed()
  }

  pub fn is_unsigned(&self) -> bool {
    !self.0.kind().is_signed()
  }

  pub fn is_integer(&self) -> bool {
    self.is_signed() || self.is_unsigned()
  }

  pub fn is_array(&self) -> bool {
    self.0.kind().is_array()
  }

  pub fn is_fn(&self) -> bool { self.0.kind().is_fn() }

  pub fn is_layout(&self) -> bool { format!("{self:?}") == "Layout" }

  
  pub fn is_struct(&self) -> bool {
    self.0.kind().is_struct() && !self.is_box() && !self.is_layout()
  }

  pub fn is_ref(&self) -> bool { self.0.kind().is_ref() }

  pub fn is_ptr(&self) -> bool { self.0.kind().is_raw_ptr() }

  pub fn is_box(&self) -> bool { self.0.kind().is_box() }

  /// `Box` is also a ptr by our semantic
  pub fn is_any_ptr(&self) -> bool {
    self.is_ref() || self.is_ptr() || self.is_box()
  }

  pub fn array_domain(&self) -> Type {
    if let TyKind::RigidTy(r) = self.0.kind() {
      if let RigidTy::Array(t, ..) = r {
        return Type::from(t);
      }
    }
    panic!("Wrong struct type");
  }

  pub fn fn_def(&self) -> (FnDef, GenericArgs) {
    assert!(self.is_fn());
    let kind = self.0.kind();
    let _def = kind.fn_def().unwrap();
    (_def.0, _def.1.clone())
  }

  pub fn struct_name(&self) -> NString {
    assert!(self.is_struct());
    if let TyKind::RigidTy(r) = self.0.kind() {
      if let RigidTy::Adt(adt, _) = r {
        return NString::from(adt.trimmed_name());
      }
    }
    panic!("Wrong struct type");
  }

  pub fn struct_def(&self) -> StructDef {
    assert!(self.is_struct());
    let mut def = (self.struct_name(), Vec::new());
    if let TyKind::RigidTy(r) = self.0.kind() {
      if let RigidTy::Adt(adt, _) = r {
        for field in adt.variants()[0].fields() {
          def.1.push((NString::from(field.name.clone()), Type::from(field.ty())));
        }
      }
    }
    assert!(!def.1.is_empty());
    def
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
      RigidTy::Adt(def, _) => {
        let name = def.trimmed_name();
        let mut fields = Vec::new();
        if name != "Box" && name != "Layout" {
          for field in def.variants()[0].fields() {
            fields.push(Type::from(field.ty()));
          }
        }
        let ftypes = 
          fields
            .iter()
            .map(|t| format!("{t:?}"))
            .collect::<Vec<String>>()
            .join(", ");
        if name != "Box" && name != "Layout" {
          write!(f, "{name} {{ {ftypes} }}")
        } else {
          write!(f, "{name}")
        }
      },
      RigidTy::Array(ty, c) => {
        write!(f, "Array_{ty:?}-{c:?}")
      },
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
          Err(Error).expect("data must be empty")
        }
      }
      _ => Err(Error).expect(format!("Do not support type {rigid:?}").as_str()),
    }
  }
}

impl From<Ty> for Type {
  fn from(value: Ty) -> Self {
    assert!(matches!(value.kind(), TyKind::RigidTy(_)));
    Type(value)
  }
}

impl ToString for Type {
  fn to_string(&self) -> String {
    format!("{self:?}")
  }
}