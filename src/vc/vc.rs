
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::slice::{Iter, IterMut};

use crate::expr::context::ExprCtx;
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
      VcKind::Assert(_, cond)  |
      VcKind::Assume(cond) => write!(f, "{cond:?}"),
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
  vconds: Vec<Vc>,
}

impl VCSystem {
  pub fn assign(&mut self, lhs: Expr, mut rhs: Expr) {
    // println!("ASSIGN: {lhs:?} = {rhs:?}");
    self.vconds.push(Vc::new(VcKind::Assign(lhs, rhs)));
  }

  pub fn assert(&mut self, property: NString, cond: Expr) {
    println!("ASSERT: {cond:?}");
    self.vconds.push(Vc::new(VcKind::Assert(property, cond)));
  }
  
  pub fn assume(&mut self, cond: Expr) {
    self.vconds.push(Vc::new(VcKind::Assume(cond)));
  }

  pub fn iter(&self) -> Iter<'_, Vc> {
    self.vconds.iter()
  }

  pub fn iter_mut(&mut self) -> IterMut<'_, Vc> {
    self.vconds.iter_mut()
  }
}

impl Debug for VCSystem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let eqs =
      self
        .vconds
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