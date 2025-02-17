
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::slice::{Iter, IterMut};

use crate::expr::expr::*;

#[derive(Clone)]
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

#[derive(Clone)]
pub struct Vc {
  guard: Expr,
  kind: VcKind,
  is_sliced: bool,
}

impl Vc {
  pub fn new(guard: Expr, kind: VcKind) -> Self {
    Vc { guard, kind, is_sliced: false }
  }

  pub fn guard(&self) -> Expr { self.guard.clone() }

  pub fn kind(&self) -> VcKind { self.kind.clone() }
}

impl Debug for Vc {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}{:?}",
      if !self.guard.is_true() {
        format!("{:?} => ", self.guard)
      } else { "".to_string() },
      self.kind
      )
  }
}

/// Verification Condition System. The output of symbolic execution.
/// Used for encoding SMT formulas.
#[derive(Default)]
pub struct VCSystem {
  vconds: Vec<Vc>,
}

impl VCSystem {
  pub fn assign(&mut self, guard: Expr, lhs: Expr, mut rhs: Expr) {
    println!("ASSIGN: {lhs:?} = {rhs:?}");
    self.vconds.push(Vc::new(guard, VcKind::Assign(lhs, rhs)));
  }

  pub fn assert(&mut self, cond: Expr) {
    self.vconds.push(
      Vc::new(
        cond.ctx.constant_bool(true),
        VcKind::Assert(cond))
      );
  }
  
  pub fn assume(&mut self, cond: Expr) {
    self.vconds.push(
      Vc::new(
        cond.ctx.constant_bool(true),
        VcKind::Assume(cond))
      );
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