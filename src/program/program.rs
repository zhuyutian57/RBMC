
use std::cmp::max;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::*;

use num_bigint::BigInt;
use num_bigint::Sign;
use stable_mir::mir::mono::StaticDef;
use stable_mir::ty::FnDef;
use stable_mir::ty::UintTy;
use stable_mir::*;
use stable_mir::mir::*;
use stable_mir::target::*;

use crate::expr::ty::*;
use crate::symbol::nstring::NString;
use super::function::*;

pub struct Program {
  crate_name: NString,
  static_variables: Vec<StaticDef>,
  functions: Vec<Function>,
  idx: HashMap<NString, FunctionIdx>,
}

impl Program {
  pub fn new(_crate: Crate) -> Self {
    let mut functions = Vec::new();
    let mut idx = HashMap::new();
    for def in _crate.fn_defs() {
      if def.trimmed_name() == "main" {
        functions.push(Function::new(def.clone()));
      }
    }
    assert!(!functions.is_empty());
    for def in _crate.fn_defs() {
      if def.trimmed_name() == "main" { continue; }
      functions.push(Function::new(def));
    }
    for (i, function)
      in functions.iter_mut().enumerate() {
      idx.insert(function.name().clone(), i);
    }
    Program {
      crate_name: _crate.name.clone().into(),
      static_variables: _crate.statics(),
      functions,
      idx
    }
  }

  pub fn static_variables(&self) -> &Vec<StaticDef> {
    &self.static_variables
  }

  pub fn function(&self, i: FunctionIdx) -> &Function {
    assert!(i < self.functions.len());
    &self.functions[i]
  }

  pub fn size(&self) -> usize { self.functions.len() }

  pub fn function_idx(&self, name: NString) -> FunctionIdx {
    *self.idx.get(&name).expect("Not exists")
  }

  pub fn contains_function(&self, name: NString) -> bool {
    self.idx.contains_key(&name)
  }

  pub fn show(&self) {
    let target = MachineInfo::target();
    println!(
      "Crate:{:?}, Endian:{}, MachineSize:{}",
      self.crate_name,
      match target.endian {
        Endian::Little => "Little",
        _ => "Big",
      },
      target.pointer_width.bytes()
    );
    for function in self.functions.iter() {
      println!("--->>> Function: {:?}", function.name());
      function
        .body()
        .dump(&mut stdout().lock(), &function.name().to_string())
        .unwrap();
      println!("<<<--- End: {:?}\n", function.name());
    }
  }
}

pub(crate) fn read_target_integer(bytes: &[u8]) -> BigInt {
  match MachineInfo::target().endian {
    Endian::Big => BigInt::from_signed_bytes_be(bytes),
    Endian::Little => BigInt::from_signed_bytes_le(bytes),
  }
}

pub fn bigint_to_u64(bigint: &BigInt) -> u64 {
  if bigint == &BigInt::ZERO { return 0; }
  let (sign, digits) = bigint.to_u64_digits();
  assert!(sign == Sign::NoSign || sign == Sign::Plus);
  assert!(digits.len() == 1);
  digits[0]
}

pub fn bigint_to_usize(bigint: &BigInt) -> usize {
  bigint_to_u64(bigint) as usize
}