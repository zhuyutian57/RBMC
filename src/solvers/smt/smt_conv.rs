
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::Program;
use crate::solvers::solver::Result;
use crate::NString;

pub(crate) trait Solve {
  fn init(&mut self, program: &Program);
  fn assert_expr(&self, expr: Expr);
  fn push(&self);
  fn pop(&self, n: u32);
  fn reset(&self);
  fn dec_check(&self) -> Result;
}

pub(crate) trait Convert<Sort, Ast> {
  fn convert_sort(&self, ty: Type) -> Sort {
    if ty.is_bool() { return self.mk_bool_sort(); }
    if ty.is_integer() { return self.mk_int_sort(); }
    if ty.is_struct() { panic!("mk tuple"); }
    panic!("Not support yet");
  }

  fn convert_ast(&self, expr: Expr) -> Ast {

    // cache SMT ast

    // convert sub exprs firstly
    let mut args: Vec<Ast> = Vec::new();
    if let Some(sub_exrps) = expr.sub_exprs() {
      for e in sub_exrps {
        args.push(self.convert_ast(e));
      }
    }

    // let mut a;
    if expr.is_terminal() {

    }

    if expr.is_address_of() {

    }

    if expr.is_binary() {

    }

    if expr.is_unary() {

    }

    if expr.is_cast() {

    }

    if expr.is_object() {

    }

    if expr.is_index_of() {

    }

    if expr.is_ite() {

    }

    if expr.is_same_object() {

    }

    if expr.is_store() {

    }

    todo!()
  }

  // sort
  fn mk_bool_sort(&self) -> Sort;
  fn mk_int_sort(&self) -> Sort;
  fn mk_array_sort(&self, domain: Sort, range: Sort) -> Sort;

  // constant
  fn mk_smt_bool(&self, b: bool) -> Ast;
  fn mk_smt_int(&self, i: u128) -> Ast; // TODO: set bigint
  
  // variable
  fn mk_bool_var(&self, name: NString) -> Ast;
  fn mk_int_var(&self, name: NString) -> Ast;
  fn mk_array_var(&self, name: NString, domain: Sort, range: Sort) -> Ast;

  // expr
  fn mk_add(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_sub(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_mul(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_div(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_eq(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_ne(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_ge(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_gt(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_le(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_lt(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_and(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_or(&self, lhs: Ast, rhs: Ast) -> Ast;
  fn mk_not(&self, operand: Ast) -> Ast;
  fn mk_implies(&self, cond: Ast, conseq: Ast) -> Ast;

}