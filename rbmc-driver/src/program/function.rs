use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;

use stable_mir::mir::mono::Instance;
use stable_mir::mir::*;
use stable_mir::ty::FnDef;
use stable_mir::*;

use crate::expr::ty::*;
use crate::symbol::nstring::NString;

pub type FunctionIdx = usize;
pub type Args = Vec<Local>;
pub type Pc = BasicBlockIdx;

/// A wrapper for functiom item in MIR
pub struct Function {
    name: NString,
    args: Args,
    body: Body,
    ty: Type,
    /// Record the locals without StorageLive
    _local_alive: HashSet<Local>,
    /// Record loop entries
    _loop_entries: HashSet<Pc>,
}

impl Function {
    fn init(&mut self) {
        self.reconstruct_body();
        self.init_locals_without_storagelive();
        self.init_loop_entries();
    }

    fn init_locals_without_storagelive(&mut self) {
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
    }

    fn init_loop_entries(&mut self) {
        for i in 0..self.body.blocks.len() {
            match &self.basicblock(i).terminator.kind {
                TerminatorKind::Goto { target } => {
                    if *target < i {
                        // Back edge
                        self._loop_entries.insert(*target);
                    }
                }
                _ => {},
            }
        }
    }

    fn reconstruct_body(&mut self) {
        let mut n: usize = self.body.blocks.len();
        let mut bb_map = HashMap::<usize, usize>::new();
        let ret_blocks = self.body.blocks
            .iter()
            .enumerate()
            .filter(
                |&(_, bb)| matches!(bb.terminator.kind, TerminatorKind::Return)
            )
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        for (i, &bb) in ret_blocks.iter().enumerate() {
            bb_map.insert(bb, n - 1 - i);
        }
        if ret_blocks.len() > 1 { n += 1; }

        let mut remaining_blocks = self.body.blocks
            .iter()
            .enumerate()
            .filter(
                |&(_, bb)| !matches!(bb.terminator.kind, TerminatorKind::Return)
            )
            .map(|(i, _)| i)
            .collect::<HashSet<_>>();

        self.reconstruct_blocks(0, 0, remaining_blocks, &mut bb_map);

        let mut new_blocks = Vec::new();
        let mut bb_map_vec = bb_map.clone().into_iter().collect::<Vec<_>>();
        bb_map_vec.sort_by(|a, b| a.1.cmp(&b.1));
        for (bb, new_bb) in bb_map_vec {
            assert!(new_bb == new_blocks.len());
            let mut new_block = self.body.blocks[bb].clone();
            match &mut new_block.terminator.kind {
                TerminatorKind::Goto { target } |
                TerminatorKind::Drop { target, .. } |
                TerminatorKind::Assert { target, .. }
                    => *target = *bb_map.get(target).unwrap(),
                TerminatorKind::Call { target, .. }
                    => if let Some(t) = target { *t = *bb_map.get(t).unwrap(); }
                TerminatorKind::SwitchInt { targets, .. } => {
                    let mut new_branches = Vec::new();
                    for (discr, successor) in targets.branches() {
                        let new_successor = *bb_map.get(&successor).unwrap();
                        new_branches.push((discr, new_successor));
                    }
                    let new_otherwise = *bb_map.get(&targets.otherwise()).unwrap();
                    *targets = SwitchTargets::new(new_branches, new_otherwise);
                }
                _ => {},
            }
            if ret_blocks.len() > 1 &&
                matches!(new_block.terminator.kind, TerminatorKind::Return) {
                new_block.terminator.kind = TerminatorKind::Goto { target: n - 1 };
            }
            new_blocks.push(new_block);
        }
        // New return block
        if ret_blocks.len() > 1 {
            let ret_span = self.basicblock(ret_blocks[0]).terminator.span;
            new_blocks.push(
                BasicBlock {
                    statements: vec![],
                    terminator: Terminator {
                        kind: TerminatorKind::Return,
                        span: ret_span
                    }
                }
            );
        }
        self.body.blocks = new_blocks;
    }

    fn reconstruct_blocks(
        &mut self,
        entry: BasicBlockIdx,
        prefix_nodes: usize,
        remaining_blocks: HashSet<BasicBlockIdx>,
        bb_map: &mut HashMap<usize, usize>
    ) {
        if bb_map.contains_key(&entry) { return; }
        let mut back_edge_src = None;
        let mut predecessors: HashMap<usize, HashSet<usize>> = HashMap::new();
        for &i in remaining_blocks.iter() {
            for j in self.body.blocks[i].terminator.successors() {
                if !remaining_blocks.contains(&j) { continue; }
                predecessors.entry(j).or_default();
                predecessors.entry(j).and_modify(|x| {
                    x.insert(i);
                });
                if j == entry { back_edge_src = Some(i); }
            }
        }
        // Compute the SCC from entry.
        let mut suffix_remaining_blocks = remaining_blocks;
        let mut scc = HashSet::new();
        scc.insert(entry);
        suffix_remaining_blocks.remove(&entry);
        if let Some(src) = back_edge_src {
            scc.insert(src);
            let mut work = VecDeque::new();
            work.push_back(src);;
            while !work.is_empty() {
                let j = work.pop_front().unwrap();
                if !suffix_remaining_blocks.contains(&j) { continue; }
                scc.insert(j);
                suffix_remaining_blocks.remove(&j);
                for &i in predecessors.get(&j).unwrap() {
                    if !suffix_remaining_blocks.contains(&i) { continue; }
                    work.push_back(i);
                }
            }
        }
        // Reconstruct the remaining basic blocks.
        if !suffix_remaining_blocks.is_empty() {
            let suffix_entry = *suffix_remaining_blocks.iter().min().unwrap();
            let suffix_prefix_nodes = prefix_nodes + scc.len();
            self.reconstruct_blocks(
                suffix_entry, suffix_prefix_nodes, suffix_remaining_blocks, bb_map
            );
        }
        // Reconstruct the current SCC.
        let mut scc_remaining_blocks = scc.clone();
        // Fix positions of the bound of SCC.
        bb_map.insert(entry, prefix_nodes);
        scc_remaining_blocks.remove(&entry);
        if let Some(src) = back_edge_src {
            bb_map.insert(src, prefix_nodes + scc.len() - 1);
            scc_remaining_blocks.remove(&src);
        }
        // Recursively reconstruct the body of SCC.
        if !scc_remaining_blocks.is_empty() {
            let scc_entry = *scc_remaining_blocks.iter().min().unwrap();
            let scc_prefix_nodes = prefix_nodes + 1;
            self.reconstruct_blocks(
                scc_entry, scc_prefix_nodes, scc_remaining_blocks, bb_map
            );
        }
    }

    pub fn name(&self) -> NString {
        self.name
    }

    pub fn ty(&self) -> Type {
        self.ty
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

    pub fn is_loop_entry(&self, pc: Pc) -> bool {
        self._loop_entries.contains(&pc)
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

impl From<(NString, Body, Type)> for Function {
    fn from(value: (NString, Body, Type)) -> Self {
        let mut function = Function {
            name: NString::from(value.0),
            args: (1..value.1.arg_locals().len() + 1).collect(),
            body: value.1,
            ty: value.2,
            _local_alive: HashSet::new(),
            _loop_entries: HashSet::new(),
        };
        function.init();
        function
    }
}

impl From<&FnDef> for Function {
    fn from(value: &FnDef) -> Self {
        assert!(value.has_body());
        let ty = Type::from(value.ty());
        Function::from((value.trimmed_name().into(), value.body().unwrap(), ty))
    }
}

impl From<&Instance> for Function {
    fn from(value: &Instance) -> Self {
        assert!(value.has_body());
        let ty = Type::from(value.ty());
        Function::from((value.trimmed_name().into(), value.body().unwrap(), ty))
    }
}
