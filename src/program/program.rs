
use std::collections::HashMap;
use std::io::*;
use std::rc::Rc;

use stable_mir::*;
use stable_mir::mir::*;
use stable_mir::target::*;
use stable_mir::ty::*;

use crate::expr::context::*;
use crate::expr::ty::*;
use crate::symbol::nstring::NString;

pub type FunctionIdx = usize;
pub type StmtIdx = usize;

pub type Decl = (Type, Mutability);

/// A wrapper for functiom item in MIR
#[derive(Debug)]
pub struct Function {
  name: NString,
  locals: Vec<Decl>,
  body: Body,
}

impl Function {
  pub fn new(item: CrateItem) -> Self {
    assert!(item.kind() == ItemKind::Fn);
    Function {
      name: NString::from(item.name()),
      locals: Vec::new(),
      body: item.body(),
    }
  }

  pub fn init_locals(&mut self, ctx: ExprCtx) {
    for local in self.body.locals() {
      let local_ty  = Type::from(local.ty);
      self.locals.push((local_ty, local.mutability));
    }
  }

  pub fn name(&self) -> NString { self.name }

  pub fn body(&self) -> &Body { &self.body }

  pub fn locals(&self) -> &Vec<Decl> { &self.locals }

  pub fn size(&self) -> usize { self.body.blocks.len() }

  pub fn local_decl(&self, local: Local) -> &Decl {
    assert!(local < self.locals.len());
    &self.locals[local]
  }

  pub fn basicblock(&self, i: usize) -> &BasicBlock {
    assert!(i < self.body.blocks.len());
    &self.body.blocks[i]
  }

  pub fn statement(&self, bb: BasicBlockIdx, stmt: usize) -> &Statement {
    assert!(bb < self.body.blocks.len());
    assert!(stmt < self.body.blocks[bb].statements.len());
    &self.body.blocks[bb].statements[stmt]
  }

  pub fn terminator(&self, bb: BasicBlockIdx) -> &Terminator {
    assert!(bb < self.body.blocks.len());
    &self.body.blocks[bb].terminator
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
  target: MachineInfo,
  functions: Vec<Function>,
  idx: HashMap<NString, FunctionIdx>,
}

impl Program {
  pub fn new(
    _crate: NString,
    target: MachineInfo,
    items: CrateItems,
    ctx: ExprCtx,
  ) -> Self {
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
      functions.push(Function::new(item));
    }
    for (i, function) in functions.iter_mut().enumerate() {
      function.init_locals(ctx.clone());
      idx.insert(function.name.clone(), i);
    }
    Program { _crate, target, functions, idx }
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

  pub fn is_little_endian(&self) -> bool {
    matches!(self.target.endian, Endian::Little)
  }

  pub fn is_big_endian(&self) -> bool { !self.is_little_endian() }

  pub fn show(&self) {
    println!(
      " Crate:{:?}, Endian:{}, MachineSize:{}",
      self._crate,
      match self.target.endian {
        Endian::Little => "Little",
        _ => "Big",
      },
      self.target.pointer_width.bytes()
    );
    for function in self.functions.iter() {
      println!("\n --->> Function: {:?}", function.name());
      for local in function.locals().iter() {
        println!("{:?}", local);
      }
      function
        .body()
        .dump(&mut stdout().lock(), &function.name().to_string())
        .unwrap();
    }
  }
}

macro_rules! READ_INT {
  ($ty:ident, $cond:expr, $bytes:ident) => {
    {
      let buf = $bytes.try_into().unwrap();
      let i = 
        if $cond {
          $ty::from_le_bytes(buf)
        } else {
          $ty::from_be_bytes(buf)
        } as i128;
      Ok((i < 0, i.abs() as u128))
    }
  };
}

macro_rules! READ_UINT {
  ($ty:ident, $cond:expr, $bytes:ident) => {
    {
      let buf = $bytes.try_into().unwrap();
      Ok(
        (false,
          if $cond {
            $ty::from_le_bytes(buf)
          } else {
            $ty::from_be_bytes(buf)
          } as u128)
      )
    }
  };
}

pub(crate) fn read_target_integer(
  bytes: &[u8],
  is_signed: bool,
  is_little_endian: bool,
) -> (bool, u128) {
  match is_signed {
    true => {
      match bytes.len() {
        1 => READ_INT!(i8, is_little_endian, bytes),
        2 => READ_INT!(i16, is_little_endian, bytes),
        4 => READ_INT!(i32, is_little_endian, bytes),
        8 => READ_INT!(i64, is_little_endian, bytes),
        16 => READ_INT!(i128, is_little_endian, bytes),
        _ => Err("Wrong bytes"),
      }
    },
    false => {
      match bytes.len() {
        1 => READ_UINT!(u8, is_little_endian, bytes),
        2 => READ_UINT!(u16, is_little_endian, bytes),
        4 => READ_UINT!(u32, is_little_endian, bytes),
        8 => READ_UINT!(u64, is_little_endian, bytes),
        16 => READ_UINT!(u128, is_little_endian, bytes),
        _ => Err("Wrong bytes"),
      }
    },
  }.expect("Wrong bytes")
}