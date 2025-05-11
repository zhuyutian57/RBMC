use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::slice::{Iter, IterMut};

use stable_mir::ty::Span;

use crate::expr::context::ExprCtx;
use crate::expr::expr::*;
use crate::symbol::nstring::NString;

#[derive(Clone)]
pub enum StepKind {
    Assign(Expr, Expr),
    Assert(NString, Expr),
    Assume(Expr),
}

impl Debug for StepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepKind::Assign(lhs, rhs) => write!(f, "{lhs:?} = {rhs:?}"),
            StepKind::Assert(msg, cond) => write!(f, "{msg:?}\n    ASSERT: {cond:?}"),
            StepKind::Assume(cond) => write!(f, "{cond:?}"),
        }
    }
}

#[derive(Clone)]
pub struct SSAStep {
    pub kind: StepKind,
    pub span: Option<Span>,
    pub is_sliced: bool,
}

impl SSAStep {
    pub fn new(kind: StepKind, span: Option<Span>) -> Self {
        SSAStep { kind, span, is_sliced: false }
    }

    pub fn is_assign(&self) -> bool {
        matches!(self.kind, StepKind::Assign(..))
    }

    pub fn is_assert(&self) -> bool {
        matches!(self.kind, StepKind::Assert(..))
    }

    pub fn is_assume(&self) -> bool {
        matches!(self.kind, StepKind::Assume(..))
    }

    pub fn msg(&self) -> NString {
        if let StepKind::Assert(msg, _) = &self.kind {
            return *msg;
        }
        panic!("Not assertion")
    }

    pub fn cond(&self) -> Expr {
        match &self.kind {
            StepKind::Assert(_, c) | StepKind::Assume(c) => c.clone(),
            _ => panic!("Not assert or assume"),
        }
    }
}

impl Debug for SSAStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

/// Verification Condition System. The output of symbolic execution.
/// Used for encoding SMT formulas.
pub struct VCSystem {
    _ctx: ExprCtx,
    pub(super) ssa_steps: Vec<SSAStep>,
    pub(super) asserts_map: HashMap<usize, usize>,
}

impl VCSystem {
    pub fn new(_ctx: ExprCtx) -> Self {
        VCSystem { _ctx, ssa_steps: Vec::new(), asserts_map: HashMap::new() }
    }

    pub fn num_step(&self) -> usize {
        self.ssa_steps.len()
    }

    pub fn num_asserts(&self) -> usize {
        self.asserts_map.len()
    }

    pub fn assign(&mut self, lhs: Expr, rhs: Expr, span: Option<Span>) {
        self.ssa_steps.push(SSAStep::new(StepKind::Assign(lhs, rhs), span));
    }

    pub fn assert(&mut self, msg: NString, cond: Expr, span: Option<Span>) {
        self.asserts_map.insert(self.asserts_map.len(), self.ssa_steps.len());
        self.ssa_steps.push(SSAStep::new(StepKind::Assert(msg, cond), span));
    }

    pub fn assume(&mut self, cond: Expr) {
        self.ssa_steps.push(SSAStep::new(StepKind::Assume(cond), None));
    }

    pub fn nth(&self, n: usize) -> SSAStep {
        assert!(n < self.ssa_steps.len());
        self.ssa_steps[n].clone()
    }

    pub fn nth_assertion(&self, n: usize) -> SSAStep {
        assert!(n < self.asserts_map.len());
        self.nth(*self.asserts_map.get(&n).unwrap())
    }

    pub fn set_nth_assertion(&mut self, n: usize) {
        let m = *self.asserts_map.get(&n).unwrap();
        for (i, vc) in self.ssa_steps.iter_mut().enumerate() {
            if vc.is_assert() {
                vc.is_sliced = i != m;
            } else {
                vc.is_sliced = i > m;
            }
        }
    }

    pub fn iter(&self) -> Iter<'_, SSAStep> {
        self.ssa_steps.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, SSAStep> {
        self.ssa_steps.iter_mut()
    }

    pub fn show_info(&self) {
        println!(
            "Symex completed in {} steps, generating {} VCC(s)",
            self.ssa_steps.len(),
            self.asserts_map.len()
        );
    }

    pub fn show_vcc(&self) {
        let mut keys = self.asserts_map.keys().map(|k| *k).collect::<Vec<_>>();
        keys.sort();
        for i in keys.iter() {
            let m = *self.asserts_map.get(i).unwrap();
            if self.ssa_steps[m].is_sliced {
                continue;
            }
            let span = self.ssa_steps[m].span.expect("Span must exist");
            println!(
                "\nAssertion {i}: {}:{}:{}",
                span.get_filename(),
                span.get_lines().start_line,
                span.get_lines().start_col
            );
            println!("-> Check: {:?}", self.ssa_steps[m].msg());
            let mut n = 0;
            let mut cond = self._ctx._true();
            for j in 0..m {
                if self.ssa_steps[j].is_sliced {
                    continue;
                }
                if self.ssa_steps[j].is_assert() {
                    continue;
                }
                if self.ssa_steps[j].is_assume() {
                    cond = self._ctx.and(cond, self.ssa_steps[j].cond());
                    continue;
                }
                println!("#{n} {:?}", self.ssa_steps[j]);
                n += 1;
            }
            println!("-> ASSERT: {cond:?} && {:?}", self.ssa_steps[m].cond());
        }
    }
}

impl Debug for VCSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let eqs = self
            .ssa_steps
            .iter()
            .enumerate()
            .map(|(i, eq)| format!("#{i}  {eq:?}\n"))
            .collect::<String>();
        write!(f, "Verification Conditions:\n{eqs}")
    }
}

pub type VCSysPtr = Rc<RefCell<VCSystem>>;
