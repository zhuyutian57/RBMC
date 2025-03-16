
use std::collections::HashSet;

use stable_mir::ty::FnDef;
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
  locals_without_storage: Vec<Local>,
}

impl Function {
  pub fn new(def: FnDef) -> Self {
    let mut locals_with_storage = HashSet::new();
    for bb in def.body().unwrap().blocks {
      for s in bb.statements {
        match s.kind {
          StatementKind::StorageLive(l) => {
            locals_with_storage.insert(l);
          },
          _ => {},
        }
      }
    }
    let mut locals_without_storage = Vec::new();
    for i in 1..def.body().unwrap().locals().len() {
      if locals_with_storage.contains(&i) { continue; }
      locals_without_storage.push(i);
    }
    Function {
      name: NString::from(def.trimmed_name()),
      args: (1..def.body().unwrap().arg_locals().len() + 1).collect(),
      body: def.body().unwrap(),
      locals_without_storage,
    }
  }

  pub fn name(&self) -> NString { self.name }

  pub fn args(&self) -> &Args { &self.args }

  pub fn locals(&self) -> &[LocalDecl] { self.body.locals() }
  
  pub fn local_decl(&self, local: Local) -> &LocalDecl {
    assert!(local < self.locals().len());
    self.body.local_decl(local).unwrap()
  }

  pub fn local_without_storage(&self) -> Vec<Local> {
    self.locals_without_storage.clone()
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