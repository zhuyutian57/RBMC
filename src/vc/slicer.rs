
use std::collections::HashSet;

use crate::expr::expr::Expr;
use super::vc::*;

#[derive(Default)]
pub struct Slicer {
  cache_expr: HashSet<Expr>,
}

impl Slicer {
  pub fn slice_nth(&mut self, vc_system: VCSysPtr, n: usize) {
    assert!(n < vc_system.borrow().num_asserts());
    for vc in vc_system.borrow_mut().iter_mut() {
      vc.is_sliced = true;
    }
    let m = *vc_system.borrow().asserts_map.get(&n).unwrap();
    vc_system.borrow_mut().vcs[m].is_sliced = false;
    for i in (0..m + 1).rev() {
      self.slice(&mut vc_system.borrow_mut().vcs[i]);
    }
  }

  pub fn slice_whole(&mut self, vc_system: VCSysPtr) {
    for vc in vc_system.borrow_mut().iter_mut() {
      vc.is_sliced = !vc.is_assert();
    }
    for vc in vc_system.borrow_mut().iter_mut().rev() {
      self.slice(vc);
    }
  }

  fn get_symbols(&mut self, expr: &Expr, is_cached: bool) -> bool {
    let mut res = false;

    if expr.is_symbol() {
      res |= self.cache_expr.contains(expr);
      self.cache_expr.insert(expr.clone());
    }

    if let Some(sub_exprs) = expr.sub_exprs() {
      for sub_expr in sub_exprs {
        res |= self.get_symbols(&sub_expr, is_cached);
      }
    }

    res
  }

  fn slice(&mut self, vc: &mut Vc) {
    match &vc.kind {
      VcKind::Assign(lhs, rhs) => {
        if self.get_symbols(lhs, false) {
          vc.is_sliced = false;
          self.get_symbols(rhs, true);
        }
      },
      VcKind::Assert(_, cond) => {
        if !vc.is_sliced {
          // If the assertiong is included, caching the symbols
          self.get_symbols(cond, true);
        }
      },
      VcKind::Assume(cond) => {
        if self.get_symbols(cond, false) {
          vc.is_sliced = false;
          self.get_symbols(cond, true);
        }
      },
    };
  }
}