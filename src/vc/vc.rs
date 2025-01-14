
use std::fmt::Debug;

use crate::expr::expr::*;

pub enum VcKind {
  Assign(Expr, Expr),
  Assert(Expr),
  Assume(Expr),
}

impl Debug for VcKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      VcKind::Assign(lhs, rhs)
        => write!(f, "{lhs:?} = {rhs:?}"),
      VcKind::Assert(cond)  |
      VcKind::Assume(cond) => write!(f, "{cond:?}"),
    }
  }
}

/// Verification Condition System. The output of symbolic execution.
/// Used for encoding SMT formulas.
#[derive(Default)]
pub struct VCSystem {
  equantions: Vec<VcKind>,
}

impl VCSystem {
  pub fn assign(&mut self, lhs: Expr, mut rhs: Expr) {
    println!("ASSIGN: {lhs:?} = {rhs:?}");
    self.equantions.push(VcKind::Assign(lhs, rhs));
  }

  pub fn assert(&mut self, cond: Expr) {
    self.equantions.push(VcKind::Assert(cond));
  }
  
  pub fn assume(&mut self, cond: Expr) {
    self.equantions.push(VcKind::Assume(cond));
  }
}

impl Debug for VCSystem {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Verification Conditions:");
    let eqs =
      self
        .equantions
        .iter()
        .enumerate()
        .map(
          |(i, eq)|
          format!("#{i}  {eq:?}\n")
        )
        .collect::<String>();
    write!(f, "{eqs}")
  }
}