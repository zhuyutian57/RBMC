
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::slice::{Iter, IterMut};

use crate::expr::expr::*;
use crate::NString;

#[derive(Clone)]
pub enum VcKind {
  Assign(Expr, Expr),
  Assert(NString, Expr),
  Assume(Expr),
}

impl Debug for VcKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      VcKind::Assign(lhs, rhs)
        => write!(f, "{lhs:?} = {rhs:?}"),
      VcKind::Assert(msg, cond)
        => write!(f, "{msg:?}\n    ASSERT: {cond:?}"),
      VcKind::Assume(cond)
        => write!(f, "{cond:?}"),
    }
  }
}

#[derive(Clone)]
pub struct Vc {
  pub kind: VcKind,
  pub is_sliced: bool,
}

impl Vc {
  pub fn new(kind: VcKind) -> Self {
    Vc { kind, is_sliced: false }
  }

  pub fn is_assign(&self) -> bool {
    matches!(self.kind, VcKind::Assign(..))
  }
  
  pub fn is_assert(&self) -> bool {
    matches!(self.kind, VcKind::Assert(..))
  }
  
  pub fn is_assume(&self) -> bool {
    matches!(self.kind, VcKind::Assume(..))
  }

  pub fn msg(&self) -> NString {
    if let VcKind::Assert(msg, _) = &self.kind {
      return *msg;
    }
    panic!("Not assertion")
  }

  pub fn cond(&self) -> Expr {
    match &self.kind {
      VcKind::Assert(_, c) |
      VcKind::Assume(c) => c.clone(),
      _ => panic!("Not assert or assume"),
    }
  }
}

impl Debug for Vc {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.kind)
  }
}

/// Verification Condition System. The output of symbolic execution.
/// Used for encoding SMT formulas.
#[derive(Default)]
pub struct VCSystem {
  pub(super) vcs: Vec<Vc>,
  pub(super) asserts_map: HashMap<usize, usize>,
}

impl VCSystem {
  pub fn num_asserts(&self) -> usize {
    self.asserts_map.len()
  }

  pub fn assign(&mut self, lhs: Expr, rhs: Expr) {
    self.vcs.push(Vc::new(VcKind::Assign(lhs, rhs)));
  }

  pub fn assert(&mut self, msg: NString, cond: Expr) {
    self.asserts_map.insert(
      self.asserts_map.len(),
      self.vcs.len()
    );
    self.vcs.push(Vc::new(VcKind::Assert(msg, cond)));
  }
  
  pub fn assume(&mut self, cond: Expr) {
    self.vcs.push(Vc::new(VcKind::Assume(cond)));
  }

  pub fn set_nth_assertion(&mut self, n: usize) {
    let m = *self.asserts_map.get(&n).unwrap();
    for (i, vc) in self.vcs.iter_mut().enumerate() {
      if vc.is_assert() { vc.is_sliced = i != m; }
      else { vc.is_sliced = i > m; }
    }
  }

  pub fn iter(&self) -> Iter<'_, Vc> {
    self.vcs.iter()
  }

  pub fn iter_mut(&mut self) -> IterMut<'_, Vc> {
    self.vcs.iter_mut()
  }

  pub fn show_vcc(&self) {
    for i in self.asserts_map.keys() {
      let m = *self.asserts_map.get(i).unwrap();
      if self.vcs[m].is_sliced { continue; }
      println!("-> {:?}", self.vcs[m].msg());
      let mut n = 0;
      for j in 0..m {
        if self.vcs[j].is_sliced { continue; }
        if self.vcs[j].is_assert() { continue; }
        println!("#{n} {:?}", self.vcs[j]);
        n += 1;
      }
      println!("-> ASSERT: {:?}\n", self.vcs[m].cond());
    }
  }

}

impl Debug for VCSystem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let eqs =
      self
        .vcs
        .iter()
        .enumerate()
        .map(
          |(i, eq)|
          format!("#{i}  {eq:?}\n")
        )
        .collect::<String>();
    write!(f, "Verification Conditions:\n{eqs}")
  }
}

pub type VCSysPtr = Rc<RefCell<VCSystem>>;