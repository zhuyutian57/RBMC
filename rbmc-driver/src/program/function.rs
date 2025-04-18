use std::collections::HashMap;
use std::collections::HashSet;

use stable_mir::mir::mono::Instance;
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
pub struct Function {
    name: NString,
    args: Args,
    body: Body,
    /// Record the locals without StorageLive
    _local_alive: HashSet<Local>,
    _loops: LoopSet,
}

impl Function {
    fn init(&mut self) {
        // Find locals without StorageLive
        for local in 1..self.locals().len() {
            let mut is_alive = true;
            for bb in &self.body.blocks {
                for st in &bb.statements {
                    match &st.kind {
                        StatementKind::StorageLive(l) => is_alive &= local != *l,
                        _ => {}
                    }
                }
            }
            if is_alive {
                self._local_alive.insert(local);
            }
        }
        // Find all loops
        let mut predecessors: HashMap<usize, HashSet<usize>> = HashMap::new();
        for i in 0..self.body.blocks.len() {
            for j in self.body.blocks[i].terminator.successors() {
                predecessors.entry(j).or_default();
                predecessors.entry(j).and_modify(|x| {
                    x.insert(i);
                });
            }
        }
        for i in 0..self.body.blocks.len() {
            for j in self.body.blocks[i].terminator.successors() {
                // Back edge
                if i > j {
                    let mut _loop = HashSet::new();
                    _loop.insert(i);
                    let mut stack = vec![i];
                    while !stack.is_empty() {
                        let n = stack.pop().unwrap();
                        if n == j { continue; }
                        if let Some(preds) = predecessors.get(&n) {
                            for pred in preds {
                                if !_loop.contains(pred) {
                                    _loop.insert(*pred);
                                    stack.push(*pred);
                                }
                            }
                        }
                    }
                    if !_loop.contains(&j) { continue; }
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

    pub fn locals_alive(&self) -> &HashSet<Local> {
        &self._local_alive
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

    pub fn show(&self) {
        self.body().dump(&mut std::io::stdout().lock(), &self.name().to_string()).unwrap();
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Function {}


impl From<(NString, Body)> for Function {
    fn from(value: (NString, Body)) -> Self {
        let mut function = Function {
            name: NString::from(value.0),
            args: (1..value.1.arg_locals().len() + 1).collect(),
            body: value.1,
            _local_alive: HashSet::new(),
            _loops: HashMap::new()
        };
        function.init();
        function
    }
}

impl From<&FnDef> for Function {
    fn from(value: &FnDef) -> Self {
        assert!(value.has_body());
        Function::from((value.trimmed_name().into(), value.body().unwrap()))
    }
}

impl From<&Instance> for Function {
    fn from(value: &Instance) -> Self {
        assert!(value.has_body());
        Function::from((value.trimmed_name().into(), value.body().unwrap()))
    }
}