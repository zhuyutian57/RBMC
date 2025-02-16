
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::NString;

pub trait SolverApi {
  fn add(&self, expr: Expr);
  fn check(&self);
}

pub trait Convert<SmtSort, SmtAst> {

  fn convert_sort(&self, ty: Expr) -> SmtSort {
    todo!()
  }

  fn convert_ast(&self, expr: Expr) -> SmtAst {

    // cache SMT ast

    // convert sub exprs firstly
    let mut args: Vec<SmtAst> = Vec::new();
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
  fn mk_bool_sort(&self) -> SmtSort;
  fn mk_int_sort(&self) -> SmtSort;
  fn mk_array_sort(&self, domain: SmtSort, range: SmtSort) -> SmtSort;

  // constant
  fn mk_smt_bool(&self, b: bool) -> SmtAst;
  fn mk_smt_int(&self, i: u128) -> SmtAst; // TODO: set bigint
  
  // variable
  fn mk_variable(&self, name: String, sort: SmtSort) -> SmtAst;

  // expr
  fn mk_add(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_sub(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_mul(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_div(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_eq(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_ne(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_ge(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_gt(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_le(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_lt(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_and(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_or(&self, lhs: SmtAst, rhs: SmtAst) -> SmtAst;
  fn mk_not(&self, operand: SmtAst) -> SmtAst;
  fn mk_implies(&self, cond: SmtAst, conseq: SmtAst) -> SmtAst;

}