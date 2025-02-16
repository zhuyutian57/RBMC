
use z3;
use z3::Sort;
use z3::SortKind;
use z3::ast::{Array, Bool, Dynamic, Int};

use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::solvers::smt::smt_conv::*;

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

impl<'ctx> SolverApi for Z3Conv<'ctx> {
  fn add(&self, expr: Expr) {
    let a = self.convert_ast(expr);
  }

  fn check(&self) {
      todo!()
  }
}

impl<'ctx> Convert<Sort<'ctx>, Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn mk_bool_sort(&self) -> Sort<'ctx> {
    todo!()
  }

  fn mk_int_sort(&self) -> Sort<'ctx> {
    todo!();
    // Sort::int(&self.ctx)
  }

  fn mk_array_sort(&self, domain: Sort<'ctx>, range: Sort<'ctx>) -> Sort<'ctx> {
    todo!();
    // Sort::array(&self.ctx, &domain, &range)
  }

  fn mk_smt_bool(&self, b: bool) -> Dynamic<'ctx> {
    todo!();
    // Dynamic::from(z3::ast::Bool::from_bool(&self.ctx, b))
  }

  fn mk_smt_int(&self, i: u128) -> Dynamic<'ctx> {
    todo!();
    // Dynamic::from(z3::ast::Int::from_u64(&self.ctx, i as u64))
  }

  fn mk_variable(&self, name: String, sort: Sort<'ctx>) -> Dynamic<'ctx> {
    todo!();
    // match sort.kind() {
    //   SortKind::Bool => Dynamic::from(z3::ast::Bool::new_const(&self.ctx, name)),
    //   SortKind::Int => Dynamic::from(z3::ast::Int::new_const(&self.ctx, name)),
    //   SortKind::Array => {
    //     let domain = sort.array_domain().unwrap();
    //     let range = sort.array_range().unwrap();
    //     Dynamic::from(z3::ast::Array::new_const(&self.ctx, name, &domain, &range))
    //   },
    //   _ => panic!("Wrong sort"),
    // }
  }

  fn mk_add(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_sub(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_mul(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_div(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_eq(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_ne(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_ge(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_gt(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_le(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_lt(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_and(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_or(&self, lhs: Dynamic<'ctx>, rhs: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_not(&self, operand: Dynamic<'ctx>) -> Dynamic<'ctx> {
      todo!()
  }

  fn mk_implies(&self, cond: Dynamic<'ctx>, conseq: Dynamic<'ctx>) -> Dynamic<'ctx> {
    todo!()
  }
}