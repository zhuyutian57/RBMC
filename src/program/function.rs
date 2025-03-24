use std::collections::HashSet;

use stable_mir::mir::*;
use stable_mir::ty::FnDef;
use stable_mir::*;

use crate::expr::ty::*;
use crate::symbol::nstring::NString;

pub type FunctionIdx = usize;
pub type Args = Vec<Local>;
pub type Pc = BasicBlockIdx;

/// A wrapper for functiom item in MIR
#[derive(Debug)]
pub struct Function {
    name: NString,
    args: Args,
    body: Body,
    _loops: HashSet<Pc>,
}

impl Function {
    pub fn new(def: FnDef) -> Self {
        let mut _loops = HashSet::new();
        let body = def.body().unwrap();
        for (i, bb) in body.blocks.iter().enumerate() {
            for succ in bb.terminator.successors() {
                // Back-Edge
                if succ <= i {
                    _loops.insert(succ);
                }
            }
        }
        Function {
            name: NString::from(def.trimmed_name()),
            args: (1..def.body().unwrap().arg_locals().len() + 1).collect(),
            body,
            _loops,
        }
    }

    pub fn name(&self) -> NString {
        self.name
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn locals(&self) -> &[LocalDecl] {
        self.body.locals()
    }

    pub fn local_decl(&self, local: Local) -> &LocalDecl {
        assert!(local < self.locals().len());
        self.body.local_decl(local).unwrap()
    }

    pub fn local_type(&self, local: Local) -> Type {
        Type::from(self.local_decl(local).ty)
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn size(&self) -> usize {
        self.body.blocks.len()
    }

    pub fn basicblock(&self, i: usize) -> &BasicBlock {
        assert!(i < self.body.blocks.len());
        &self.body.blocks[i]
    }

    pub fn is_loop_bb(&self, pc: Pc) -> bool {
        self._loops.contains(&pc)
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
