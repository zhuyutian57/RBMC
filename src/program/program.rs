
use std::collections::HashMap;
use std::io::*;

use stable_mir::*;
use stable_mir::mir::*;
use stable_mir::target::*;

use crate::expr::ty::*;
use crate::symbol::nstring::NString;

pub type FunctionIdx = usize;
pub type Args = Vec<Local>;

/// A wrapper for functiom item in MIR
#[derive(Debug)]
pub struct Function {
  name: NString,
  args: Args,
  body: Body,
}

impl Function {
  pub fn new(item: CrateItem) -> Self {
    assert!(item.kind() == ItemKind::Fn);
    Function {
      name: NString::from(item.name()),
      args: (1..item.body().arg_locals().len() + 1).collect(),
      body: item.body(),
    }
  }

  pub fn name(&self) -> NString { self.name }

  pub fn args(&self) -> &Args { &self.args }

  pub fn locals(&self) -> &[LocalDecl] { self.body.locals() }
  
  pub fn local_decl(&self, local: Local) -> &LocalDecl {
    assert!(local < self.locals().len());
    self.body.local_decl(local).unwrap()
  }

  pub fn local_type(&self, local: Local) -> Type {
    Type::from(self.local_decl(local).ty)
  }

  pub fn body(&self) -> &Body { &self.body }

  pub fn size(&self) -> usize { self.body.blocks.len() }

  pub fn basicblock(&self, i: usize) -> &BasicBlock {
    assert!(i < self.body.blocks.len());
    &self.body.blocks[i]
  }

  pub fn operand_type(&self, operand: &Operand) -> Type {
    Type::from(operand.ty(self.body.locals()).expect("Wrong operand"))
  }

  pub fn rvalue_type(&self, rvalue: &Rvalue) -> Type {
    Type::from(rvalue.ty(self.body.locals()).expect("Wrong rvalue"))
  }

}

impl PartialEq for Function {
  fn eq(&self, other: &Self) -> bool {
    self.name == other.name
  }
}

impl Eq for Function {}

pub struct Program {
  _crate: NString,
  functions: Vec<Function>,
  idx: HashMap<NString, FunctionIdx>,
}

impl Program {
  pub fn new(_crate: NString, items: CrateItems) -> Self {
    let mut functions = Vec::new();
    let mut idx = HashMap::new();
    for item in items.iter() {
      if item.name() == "main" {
        functions.push(Function::new(item.clone()));
      }
    }
    assert!(!functions.is_empty());
    for item in items {
      if item.name() == "main" { continue; }
      if !matches!(item.kind(), ItemKind::Fn) { continue; }
      functions.push(Function::new(item));
    }
    for (i, function)
      in functions.iter_mut().enumerate() {
      idx.insert(function.name.clone(), i);
    }
    Program { _crate, functions, idx }
  }

  pub fn entry_fn(&self) -> &Function { self.function(0) }

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
      " Crate:{:?}, Endian:{}, MachineSize:{}",
      self._crate,
      match target.endian {
        Endian::Little => "Little",
        _ => "Big",
      },
      target.pointer_width.bytes()
    );
    for function in self.functions.iter() {
      println!("\n --->> Function: {:?}", function.name());
      function
        .body()
        .dump(&mut stdout().lock(), &function.name().to_string())
        .unwrap();
    }
  }
}

macro_rules! READ_INT {
  ($ty:ident, $bytes:ident) => {
    {
      let buf = $bytes.try_into().unwrap();
      let i = 
        match MachineInfo::target().endian {
          Endian::Little => $ty::from_le_bytes(buf),
          _ => $ty::from_be_bytes(buf),
        } as i128;
      Ok((i < 0, i.abs() as u128))
    }
  };
}

macro_rules! READ_UINT {
  ($ty:ident, $bytes:ident) => {
    {
      let buf = $bytes.try_into().unwrap();
      Ok(
        (false,
          match MachineInfo::target().endian {
            Endian::Little => $ty::from_le_bytes(buf),
            _ => $ty::from_be_bytes(buf),
          } as u128
        )
      )
    }
  };
}

pub(crate) fn read_target_integer(
  bytes: &[u8],
  is_signed: bool,
) -> (bool, u128) {
  match is_signed {
    true => {
      match bytes.len() {
        1 => READ_INT!(i8, bytes),
        2 => READ_INT!(i16, bytes),
        4 => READ_INT!(i32, bytes),
        8 => READ_INT!(i64, bytes),
        16 => READ_INT!(i128, bytes),
        _ => Err("Wrong bytes"),
      }
    },
    false => {
      match bytes.len() {
        1 => READ_UINT!(u8, bytes),
        2 => READ_UINT!(u16, bytes),
        4 => READ_UINT!(u32, bytes),
        8 => READ_UINT!(u64, bytes),
        16 => READ_UINT!(u128, bytes),
        _ => Err("Wrong bytes"),
      }
    },
  }.expect("Wrong bytes")
}