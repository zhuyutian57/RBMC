
use std::collections::{HashMap, HashSet};

use z3;
use z3::ast::Ast;

use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::program::program::Program;
use crate::solvers::smt::smt_array::Array;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_tuple::Tuple;
use crate::solvers::solver::Result;
use crate::NString;

pub struct Z3Conv<'ctx> {
  z3_ctx: &'ctx z3::Context,
  tuples: HashMap<Type, z3::DatatypeSort<'ctx>>,
  z3_solver: z3::Solver<'ctx>,
}

impl<'ctx> Z3Conv<'ctx> {
  pub fn new(z3_ctx: &'ctx z3::Context) -> Self {
    let z3_solver = z3::Solver::new(z3_ctx);
    Z3Conv {
      z3_ctx,
      tuples: HashMap::default(),
      z3_solver
    }
  }
}

impl<'ctx> Solve for Z3Conv<'ctx> {
  fn init(&mut self, program: &Program) {
    let mut struct_types = HashSet::new();
    for func in program.functions() {
      for local in func.locals() {
        let ty = Type::from(local.ty);
        if ty.is_struct() { struct_types.insert(ty); }
      }
    }
    for struct_type in struct_types {
      let struct_def = self.mk_tuple_sort(struct_type);
      self.tuples.insert(struct_type, struct_def);
    }
  }

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

impl<'ctx> Convert<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn mk_bool_sort(&self) -> z3::Sort<'ctx> {
    z3::Sort::bool(&self.z3_ctx)
  }

  fn mk_int_sort(&self) -> z3::Sort<'ctx> {
    z3::Sort::int(&self.z3_ctx)
  }

  fn mk_array_sort(&self, domain: z3::Sort<'ctx>, range: z3::Sort<'ctx>) -> z3::Sort<'ctx> {
    z3::Sort::array(&self.z3_ctx, &domain, &range)
  }

  fn mk_smt_bool(&self, b: bool) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Bool::from_bool(&self.z3_ctx, b))
  }

  fn mk_smt_int(&self, i: u128) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Int::from_u64(&self.z3_ctx, i as u64))
  }

  fn mk_smt_var(&self, name: NString, ty: Type) -> z3::ast::Dynamic<'ctx> {
    if ty.is_bool() { return self.mk_bool_var(name); }
    if ty.is_integer() { return self.mk_int_var(name); }
    if ty.is_struct() { return self.mk_tuple_var(name, ty); }
    panic!("Not support yet")
  }

  fn mk_bool_var(&self, name: NString) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Bool::new_const(&self.z3_ctx, name.to_string()))
  }

  fn mk_int_var(&self, name: NString) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Int::new_const(&self.z3_ctx, name.to_string()))
  }

  fn mk_add(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") +
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_sub(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") -
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_mul(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") *
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_div(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") /
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_eq(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        ._eq(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_ne(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Bool::not(&self.mk_eq(lhs, rhs).as_bool().unwrap()))
  }

  fn mk_ge(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .ge(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_gt(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .gt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_le(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .le(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_lt(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .lt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_and(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Bool::and(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_or(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Bool::or(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_not(&self, operand: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(operand.as_bool().expect("operand is no bool").not())
  }

  fn mk_implies(&self, cond: z3::ast::Dynamic<'ctx>, conseq: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      cond
        .as_bool()
        .expect("cond is not bool")
        .implies(&conseq.as_bool().expect("conseq is not bool"))
    )
  }
}

impl<'ctx> Array<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn mk_array_sort(&self, ty: Type) -> z3::Sort<'ctx> {
    todo!()
  }

  fn mk_array_var(&self, name: NString, ty: Type) -> z3::ast::Dynamic<'ctx> {
    todo!()
  }
}

impl<'ctx> Tuple<z3::DatatypeSort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn mk_tuple_sort(&self, ty: Type) -> z3::DatatypeSort<'ctx> {
    assert!(ty.is_struct());
    let def = ty.struct_def();

    let dt_name = "_struct_".to_string() + def.0.to_string().as_str();
    let mut fields = Vec::new();
    for field in def.1 {
      let accessor =
        z3::DatatypeAccessor::Sort(self.convert_sort(field.1));
      fields.push((field.0.as_str(), accessor));
    }

    z3::DatatypeBuilder::new(&self.z3_ctx, dt_name.clone())
      .variant(dt_name.as_str(), fields)
      .finish()
  }

  fn mk_tuple_var(&self, name: NString, ty: Type) -> z3::ast::Dynamic<'ctx> {
          todo!()
  }
}