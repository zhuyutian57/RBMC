
use crate::expr::constant::Constant;
use crate::expr::constant::Sign;
use crate::expr::expr::*;
use crate::expr::op::*;
use crate::expr::ty::*;
use crate::program::program::Program;
use crate::solvers::solver::Result;
use crate::NString;

pub(crate) trait SmtSolver {
  fn init(&mut self, program: &Program);
  fn assert_assign(&mut self, lhs: Expr, rhs: Expr);
  fn assert_expr(&mut self, expr: Expr);
  fn push(&self);
  fn pop(&self, n: u32);
  fn reset(&self);
  fn dec_check(&self) -> Result;
}

pub(crate) trait Convert<Sort, Ast: Clone> {
  fn cache_ast(&mut self, expr: Expr, ast: Ast);
  fn get_cache_ast(&self, expr: &Expr) -> Option<Ast>;

  fn convert_sort(&mut self, ty: Type) -> Sort {
    if ty.is_bool() { return self.mk_bool_sort(); }
    if ty.is_integer() { return self.mk_int_sort(); }

    if ty.is_any_ptr() { return self.mk_pointer_sort(); }
    if ty.is_array() {
      let domain = self.convert_sort(ty.array_domain());
      let range = self.convert_sort(ty.array_range());
      return self.mk_array_sort(&domain, &range);
    }
    if ty.is_struct() { return self.convert_tuple_sort(ty); }
    panic!("Not support yet");
  }

  fn convert_tuple_sort(&mut self, ty: Type) -> Sort;

  fn convert_ast(&mut self, expr: Expr) -> Ast {

    if let Some(a) = self.get_cache_ast(&expr) { return a; }

    // convert sub exprs firstly
    let mut args: Vec<Ast> = Vec::new();
    if !expr.is_address_of() {
      if let Some(sub_exrps) = expr.sub_exprs() {
        for e in sub_exrps {
          args.push(self.convert_ast(e));
        }
      }
    }

    let mut a = None;
    if expr.is_constant() {
      a = Some(self.convert_constant(&expr.extract_constant(), expr.ty()));
    }
    
    if expr.is_symbol() {
      let name = expr.extract_symbol().name();
      a = Some(self.convert_symbol(name, expr.ty()));
    }

    if expr.is_address_of() {
      let object = expr.extract_object();
      a = Some(self.convert_address_of(object));
    }

    if expr.is_binary() {
      let lhs = &args[0];
      let rhs = &args[1];
      a = Some(
        match expr.extract_bin_op() {
          BinOp::Add => self.mk_add(lhs, rhs),
          BinOp::Sub => self.mk_sub(lhs, rhs),
          BinOp::Mul => self.mk_mul(lhs, rhs),
          BinOp::Div => self.mk_div(lhs, rhs),
          BinOp::Eq => self.mk_eq(lhs, rhs),
          BinOp::Ne => self.mk_ne(lhs, rhs),
          BinOp::Ge => self.mk_ge(lhs, rhs),
          BinOp::Gt => self.mk_gt(lhs, rhs),
          BinOp::Le => self.mk_le(lhs, rhs),
          BinOp::Lt => self.mk_lt(lhs, rhs),
          BinOp::And => self.mk_and(lhs, rhs),
          BinOp::Or => self.mk_or(lhs, rhs),
        });
    }

    if expr.is_unary() {
      a = Some(
        match expr.extract_un_op() {
          UnOp::Not => self.mk_not(&args[0]),
          _ => panic!("Not support"),
        });
    }

    if expr.is_cast() {

    }

    if expr.is_object() {
      a = Some(self.convert_ast(expr.extract_inner_expr()));
    }

    if expr.is_index_of() {

    }

    if expr.is_ite() {
      let cond = &args[0];
      let true_value = &args[1];
      let false_value = &args[2];
      a = Some(self.mk_ite(cond, true_value, false_value));
    }

    if expr.is_same_object() {

    }

    if expr.is_store() {

    }

    match a {
      Some(ast) => {
        self.cache_ast(expr, ast.clone());
        ast
      },
      None => panic!("Not implememt: {expr:?}"),
    }
  }

  fn convert_constant(&mut self, constant: &Constant, ty: Type) -> Ast {
    match constant {
      Constant::Bool(b) => self.mk_smt_bool(*b),
      Constant::Integer(s, i) => self.mk_smt_int(*s, *i),
      Constant::Array(c, t) => {
        let domain = self.convert_sort(ty.array_domain());
        let val = self.convert_constant(&**c,*t);
        self.mk_smt_const_array(&domain, &val)
      },
      Constant::Struct(constants) => {
        let mut fields = Vec::new();
        for (c, st) in constants {
          fields.push(self.convert_constant(c, st.clone()));
        }
        self.convert_tuple(fields, ty)
      },
    }
  }

  fn convert_pointer(&self, ident: Ast, offset: Ast) -> Ast;
  fn convert_tuple(&mut self, fields: Vec<Ast>, ty: Type) -> Ast;

  fn convert_symbol(&mut self, name: NString, ty: Type) -> Ast {
    if ty.is_bool() { return self.mk_bool_symbol(name); }
    if ty.is_integer() { return self.mk_int_symbol(name); }
    if ty.is_any_ptr() {
      let sort = self.mk_pointer_sort();
      return self.mk_tuple_symbol(name, sort);
    }
    if ty.is_array() {
      let domain = self.convert_sort(ty.array_domain());
      let range = self.convert_sort(ty.array_range());
      return self.mk_array_symbol(name, domain, range);
    }
    if ty.is_struct() {
      let sort = self.convert_tuple_sort(ty);
      return self.mk_tuple_symbol(name, sort)
    }
    panic!("{ty:?} symbol is not support?")
  }

  fn convert_address_of(&mut self, object: Expr) -> Ast {
    assert!(object.is_object());
    let inner_expr = object.extract_inner_expr();
    if inner_expr.is_index_of() {
      todo!()
    }

    if inner_expr.is_symbol() {
      let ident = self.convert_object_space(inner_expr);
      let offset = self.mk_smt_int(false, 0);
      return self.convert_pointer(ident, offset);
    }

    panic!("Do not support address_of {object:?}")
  }

  fn convert_object_space(&mut self, ident: Expr) -> Ast;

  // sort
  fn mk_bool_sort(&self) -> Sort;
  fn mk_int_sort(&self) -> Sort;
  fn mk_pointer_sort(&self) -> Sort;
  fn mk_array_sort(&mut self, domain: &Sort, range: &Sort) -> Sort;

  // constant
  fn mk_smt_bool(&self, b: bool) -> Ast;
  fn mk_smt_int(&self, sign: Sign, i: u128) -> Ast;
  fn mk_smt_const_array(&self, domain: &Sort, val: &Ast) -> Ast;

  // symbol
  fn mk_bool_symbol(&self, name: NString) -> Ast;
  fn mk_int_symbol(&self, name: NString) -> Ast;
  fn mk_array_symbol(&self, name: NString, domain: Sort, range: Sort) -> Ast;
  fn mk_tuple_symbol(&self, name: NString, sort: Sort) -> Ast;

  // expr
  fn mk_add(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_sub(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_mul(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_div(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_eq(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_ne(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_ge(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_gt(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_le(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_lt(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_and(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_or(&self, lhs: &Ast, rhs: &Ast) -> Ast;
  fn mk_not(&self, operand: &Ast) -> Ast;
  fn mk_implies(&self, cond: &Ast, conseq: &Ast) -> Ast;
  fn mk_ite(&self, cond: &Ast, true_value: &Ast, false_value: &Ast) -> Ast;
}