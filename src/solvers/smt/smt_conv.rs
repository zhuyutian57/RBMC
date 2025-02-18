
use crate::expr::constant::Constant;
use crate::expr::constant::Sign;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::Program;
use crate::solvers::solver::Result;
use crate::NString;

pub(crate) trait Solve {
  fn init(&mut self, program: &Program);
  fn assert_assign(&self, lhs: Expr, rhs: Expr);
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
    if ty.is_array() {
      let domain = self.convert_sort(ty.array_domain());
      let range = self.convert_sort(ty.array_range());
      return self.mk_array_sort(domain, range);
    }
    if ty.is_struct() { self.convert_tuple_sort(ty); }
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

    let sort = self.convert_sort(expr.ty());
    if expr.is_constant() {
      return 
        match expr.extract_constant() {
          Constant::Bool(b)
            => self.mk_smt_bool(b),
          Constant::Integer(s, n)
            => self.mk_smt_int(s, n),
          Constant::Tuple(fields)
            => todo!(),
        };
    }
    
    if expr.is_symbol() {
      let name = expr.extract_symbol().name();
      return self.mk_smt_symbol(name, sort);
    }

    if expr.is_address_of() {
      todo!()
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

  // tuple
  fn convert_tuple_sort(&self, ty: Type) -> Sort;

  // sort
  fn mk_bool_sort(&self) -> Sort;
  fn mk_int_sort(&self) -> Sort;
  fn mk_array_sort(&self, domain: Sort, range: Sort) -> Sort;

  // constant
  fn mk_smt_bool(&self, b: bool) -> Ast;
  fn mk_smt_int(&self, sign: Sign, i: u128) -> Ast;

  // symbol
  fn mk_smt_symbol(&self, name: NString, sort: Sort) -> Ast;
  fn mk_bool_symbol(&self, name: NString) -> Ast;
  fn mk_int_symbol(&self, name: NString) -> Ast;
  fn mk_array_symbol(&self, name: NString, sort: Sort) -> Ast;
  fn mk_tuple_symbol(&self, name: NString, sort: Sort) -> Ast;

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