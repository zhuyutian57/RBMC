
use z3;
use z3::ast::{Array, Ast, Bool, Dynamic, Int};
use z3::Sort;
use z3::SortKind;

use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::solver::Result;

pub struct Z3Conv<'ctx> {
  z3_ctx: &'ctx z3::Context,
  z3_solver: z3::Solver<'ctx>,
}

impl<'ctx> Z3Conv<'ctx> {
  pub fn new(z3_ctx: &'ctx z3::Context) -> Self {
    let z3_solver = z3::Solver::new(z3_ctx);
    Z3Conv { z3_ctx, z3_solver }
  }
}

impl<'ctx> Decide for Z3Conv<'ctx> {
  fn assert_expr(&self, expr: Expr) {
    let a = self.convert_ast(expr);
    self.z3_solver.assert(&a.as_bool().expect("the assertion is not bool"));
  }

  fn push(&self) { self.z3_solver.push(); }
  
  fn pop(&self, n: u32) { self.z3_solver.pop(n); }
  
  fn reset(&self) { self.z3_solver.reset(); }

  fn dec_check(&self) -> Result {
    match self.z3_solver.check() {
      z3::SatResult::Unsat => Result::PUnsat,
      z3::SatResult::Unknown => Result::PUnknow,
      z3::SatResult::Sat => Result::PSat,
    }
  }
}

impl<'ctx> Convert<Sort<'ctx>, Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn mk_bool_sort(&self) -> Sort<'ctx> {
    Sort::bool(&self.z3_ctx)
  }

  fn mk_int_sort(&self) -> Sort<'ctx> {
    Sort::int(&self.z3_ctx)
  }

  fn mk_array_sort(&self, domain: Sort<'ctx>, range: Sort<'ctx>) -> Sort<'ctx> {
    Sort::array(&self.z3_ctx, &domain, &range)
  }

  fn mk_smt_bool(&self, b: bool) -> Dynamic<'ctx> {
    Dynamic::from(z3::ast::Bool::from_bool(&self.z3_ctx, b))
  }

  fn mk_smt_int(&self, i: u128) -> Dynamic<'ctx> {
    Dynamic::from(z3::ast::Int::from_u64(&self.z3_ctx, i as u64))
  }

  fn mk_bool_var(&self, name: String) -> Dynamic<'ctx> {
    Dynamic::from(z3::ast::Bool::new_const(&self.z3_ctx, name))
  }

  fn mk_int_var(&self, name: String) -> Dynamic<'ctx> {
    Dynamic::from(z3::ast::Int::new_const(&self.z3_ctx, name))
  }

  fn mk_array_var(
    &self,
    name: String,
    domain: Sort<'ctx>,
    range: Sort<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(z3::ast::Array::new_const(&self.z3_ctx, name, &domain, &range))
  }

  fn mk_add(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer") +
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_sub(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer") -
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_mul(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer") *
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_div(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer") /
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_eq(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        ._eq(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_ne(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(Bool::not(&self.mk_eq(lhs, rhs).as_bool().unwrap()))
  }

  fn mk_ge(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .ge(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_gt(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .gt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_le(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .le(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_lt(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .lt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_and(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      Bool::and(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_or(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      Bool::or(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_not(&self, operand: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(operand.as_bool().expect("operand is no bool").not())
  }

  fn mk_implies(&self, cond: Dynamic<'ctx>, conseq: Dynamic<'ctx>) -> Dynamic<'ctx> {
    Dynamic::from(
      cond
        .as_bool()
        .expect("cond is not bool")
        .implies(&conseq.as_bool().expect("conseq is not bool"))
    )
  }
}