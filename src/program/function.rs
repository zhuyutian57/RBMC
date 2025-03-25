use std::collections::HashMap;
use std::collections::HashSet;

use stable_mir::mir::*;
use stable_mir::ty::FnDef;
use stable_mir::*;

use crate::expr::ty::*;
use crate::symbol::nstring::NString;

pub type FunctionIdx = usize;
pub type Args = Vec<Local>;
pub type Pc = BasicBlockIdx;
pub type Loop = HashSet<Pc>;
pub type LoopSet = HashMap<Pc, Loop>;

/// A wrapper for functiom item in MIR
#[derive(Debug)]
pub struct Function {
    name: NString,
    args: Args,
    body: Body,
    _loops: LoopSet,
    /// Used to record loop bound for each bb in loops
    _bb_unwind_bound: HashMap<Pc, usize>,
}

impl Function {
    pub fn new(def: FnDef) -> Self {
        let body = def.body().unwrap();
        let mut function = Function {
            name: NString::from(def.trimmed_name()),
            args: (1..def.body().unwrap().arg_locals().len() + 1).collect(),
            body,
            _loops: HashMap::new(),
            _bb_unwind_bound: HashMap::new(),
        };
        function.init();
        function
    }

    fn init(&mut self) {
        let mut predecessors: HashMap<usize, HashSet<usize>> = HashMap::new();
        for i in 0..self.body.blocks.len() {
            for j in self.body.blocks[i].terminator.successors() {
                predecessors.entry(j).or_default();
                predecessors.entry(j).and_modify(|x| { x.insert(i); });
            }
        }
        // Find all loops
        for i in 0..self.body.blocks.len() {
            for j in self.body.blocks[i].terminator.successors() {
                // Back edge
                if i > j {
                    let mut _loop = HashSet::new();
                    _loop.insert(j);
                    _loop.insert(i);
                    let mut stack = vec![i];
                    while !stack.is_empty() {
                        let n = stack.pop().unwrap();
                        let preds = predecessors.get(&n).unwrap();
                        for pred in preds {
                            if !_loop.contains(pred) {
                                _loop.insert(*pred);
                                stack.push(*pred);
                            }
                        }
                    }
                    self._loops.insert(j, _loop);
                }
            }
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
        self._loops.contains_key(&pc)
    }

    pub fn get_loop(&self, pc: Pc) -> &Loop {
        assert!(self.is_loop_bb(pc));
        self._loops.get(&pc).unwrap()
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
