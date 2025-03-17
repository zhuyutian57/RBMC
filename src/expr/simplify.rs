
use std::collections::HashSet;

use num_bigint::BigInt;

use crate::symbol::symbol::*;
use crate::NString;
use super::ast::*;
use super::constant::*;
use super::context::*;
use super::expr::*;
use super::op::*;

impl Expr {
  pub fn simplify(&mut self) {
    if self.ty().is_bool() { self.to_nnf(false); }

    if self.is_binary() { self.simplify_binary(); }
    if self.is_unary() { self.simplify_unary(); }
    if self.is_ite() { self.simplify_ite(); }
    if self.is_same_object() { self.simplify_same_object(); }
  }

  fn has_implies(&self) -> bool {
    if self.is_binary() &&
      self.extract_bin_op() == BinOp::Implies {
      return true;
    }
    match self.sub_exprs() {
      Some(sub_exprs) =>
        sub_exprs
          .iter()
          .fold(false, |acc, e| acc | e.has_implies()),
      None => false,
    }
  }

  fn to_nnf(&mut self, is_not: bool) {
    if self.is_binary() {
      if self.extract_bin_op() == BinOp::Implies { return; }
      let sub_exprs = self.sub_exprs().unwrap();
      let mut lhs = sub_exprs[0].clone();
      let mut rhs = sub_exprs[1].clone();
      if lhs.ty().is_bool() { lhs.to_nnf(is_not); }
      if rhs.ty().is_bool() { rhs.to_nnf(is_not); }
      if is_not {
        *self = 
          match self.extract_bin_op() {
            BinOp::Eq => self.ctx.ne(lhs, rhs),
            BinOp::Ne => self.ctx.eq(lhs, rhs),
            BinOp::Ge => self.ctx.lt(lhs, rhs),
            BinOp::Gt => self.ctx.le(lhs, rhs),
            BinOp::Le => self.ctx.gt(lhs, rhs),
            BinOp::Lt => self.ctx.ge(lhs, rhs),
            BinOp::And => self.ctx.or(lhs, rhs),
            BinOp::Or => self.ctx.and(lhs, rhs),
            _ => panic!("Impossible"),
          };
      } else {
        *self = 
          match self.extract_bin_op() {
            BinOp::Eq => self.ctx.eq(lhs, rhs),
            BinOp::Ne => self.ctx.ne(lhs, rhs),
            BinOp::Ge => self.ctx.ge(lhs, rhs),
            BinOp::Gt => self.ctx.gt(lhs, rhs),
            BinOp::Le => self.ctx.le(lhs, rhs),
            BinOp::Lt => self.ctx.lt(lhs, rhs),
            BinOp::And => self.ctx.and(lhs, rhs),
            BinOp::Or => self.ctx.or(lhs, rhs),
            _ => panic!("Impossible"),
          };
      }
    } else if self.is_unary() {
      let mut operand = self.extract_inner_expr();
      match self.extract_un_op() {
        UnOp::Not => operand.to_nnf(!is_not),
        _ => panic!("Impossible"),
      };
      *self = operand;
    } else if is_not {
      *self = self.ctx.not(self.clone());
    }
  }

  fn simplify_args(&mut self) -> Vec<Expr> {
    let mut sub_exprs = self.sub_exprs().unwrap();
    for sub_expr in sub_exprs.iter_mut() { sub_expr.simplify(); }
    sub_exprs
  }

  fn simplify_binary(&mut self) {
    let mut sub_exprs = self.simplify_args();
    let lhs = sub_exprs[0].clone();
    let rhs = sub_exprs[1].clone();
    match self.extract_bin_op() {
      BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div
        => self.simplify_arith(lhs, rhs),
      BinOp::Eq | BinOp::Ne | BinOp::Ge | BinOp::Gt |
      BinOp::Le | BinOp::Lt
        => self.simplify_cmp(lhs, rhs),
      BinOp::And | BinOp::Or | BinOp::Implies
        => self.simplify_logic(lhs, rhs),
    };
  }

  fn simplify_arith(&mut self, lhs: Expr, rhs: Expr) {
    if lhs.is_constant() && rhs.is_constant() {
      let a = lhs.extract_constant().to_integer();
      let b = rhs.extract_constant().to_integer();
      let res =
        match self.extract_bin_op() {
          BinOp::Add => a + b,
          BinOp::Sub => a - b,
          BinOp::Mul => a * b,
          BinOp::Div => a / b,
          _ => todo!("Impossible"),
        };
      *self = self.ctx.constant_integer(res, self.ty());
    } else if lhs.is_constant() &&
      lhs.extract_constant().to_integer() == BigInt::ZERO {
      let mut res =
        match self.extract_bin_op() {
          BinOp::Add => rhs,
          BinOp::Sub => self.ctx.neg(rhs),
          BinOp::Mul | BinOp::Div
            => self.ctx.constant_integer(BigInt::ZERO, self.ty()),
          _ => todo!("Impossible"),
        };
      res.simplify();;
      *self = res;
    } else if rhs.is_constant() &&
      rhs.extract_constant().to_integer() == BigInt::ZERO {
      let mut res =
        match self.extract_bin_op() {
          BinOp::Add | BinOp::Sub => lhs,
          BinOp::Mul => self.ctx.constant_integer(BigInt::ZERO, self.ty()),
          BinOp::Div => panic!("Div zero"),
          _ => todo!("Impossible"),
        };
      res.simplify();;
      *self = res;
    } else {
      // Build with simplified sub-exprs
      *self =
        match self.extract_bin_op() {
          BinOp::Add => self.ctx.add(lhs, rhs),
          BinOp::Sub => self.ctx.sub(lhs, rhs),
          BinOp::Mul => self.ctx.mul(lhs, rhs),
          BinOp::Div => self.ctx.div(lhs, rhs),
          _ => todo!("Impossible"),
        };
    }
  }

  fn simplify_cmp(&mut self, lhs: Expr, rhs: Expr) {
    if lhs.is_constant() && rhs.is_constant() {
      let a = lhs.extract_constant().to_integer();
      let b = rhs.extract_constant().to_integer();
      let res = 
        match self.extract_bin_op() {
          BinOp::Eq => a == b,
          BinOp::Ne => a != b,
          BinOp::Ge => a >= b,
          BinOp::Gt => a > b,
          BinOp::Le => a <= b,
          BinOp::Lt => a < b,
          _ => todo!("Impossible"),
        };
      *self = self.ctx.constant_bool(res);
    } else {
      *self =
        match self.extract_bin_op() {
          BinOp::Eq => self.ctx.eq(lhs, rhs),
          BinOp::Ne => self.ctx.ne(lhs, rhs),
          BinOp::Ge => self.ctx.ge(lhs, rhs),
          BinOp::Gt => self.ctx.gt(lhs, rhs),
          BinOp::Le => self.ctx.le(lhs, rhs),
          BinOp::Lt => self.ctx.lt(lhs, rhs),
          _ => todo!("Impossible"),
        };
    }
  }

  fn simplify_logic(&mut self, lhs: Expr, rhs: Expr) {
    match self.extract_bin_op() {
      BinOp::And => {
        if lhs.is_true() {
          self.id = rhs.id;
        } else if rhs.is_true() {
          self.id = lhs.id;
        } else if lhs.is_false() || rhs.is_false() {
          self.id = Context::FALSE_ID;
        } else if lhs == rhs {
          self.id = lhs.id;
        } else {
          let mut not_rhs = self.ctx.not(rhs.clone());
          not_rhs.simplify();
          if lhs == not_rhs { *self = self.ctx._false(); }
          else { *self = self.ctx.and(lhs, rhs); }
        }
      },
      BinOp::Or => {
        if lhs.is_false() {
          self.id = rhs.id;
        } else if rhs.is_false() {
          self.id = lhs.id;
        } else if lhs.is_true() || rhs.is_true() {
          self.id = Context::TRUE_ID;
        } else if lhs == rhs {
          self.id = lhs.id;
        } else {
          let mut not_rhs = self.ctx.not(rhs.clone());
          not_rhs.simplify();
          if lhs == not_rhs { *self = self.ctx._true(); }
          else { *self = self.ctx.or(lhs, rhs); }
        }
      },
      BinOp::Implies => {
        if lhs.is_false() || rhs.is_true() {
          self.id = Context::TRUE_ID;
        } else if lhs.is_true() || rhs.is_false() {
          self.id = Context::FALSE_ID;
        } else if lhs == rhs {
          self.id = Context::TRUE_ID;
        } else {
          *self = self.ctx.implies(lhs, rhs);
        }
      },
      _ => todo!("Impossible"),
    };
  }

  fn simplify_unary(&mut self) {
    let mut sub_exprs = self.simplify_args();
    let operand = &sub_exprs[0];
    match self.extract_un_op() {
      UnOp::Not | UnOp::Neg => {
        if operand.is_unary() &&
           operand.extract_un_op() == self.extract_un_op() {
          self.id = operand.extract_inner_expr().id;
        }
      },
      _ => todo!("Not support"),
    }
  }

  fn simplify_ite(&mut self) {
    let mut sub_exprs = self.simplify_args();
    let cond = sub_exprs[0].clone();
    let true_value = sub_exprs[1].clone();
    let false_value = sub_exprs[2].clone();
    if cond.is_true() {
      self.id = true_value.id;
    } else if cond.is_false() {
      self.id = false_value.id;
    } else {
      *self = self.ctx.ite(cond, true_value, false_value);
    }
  }

  fn simplify_same_object(&mut self) {
    let mut sub_exprs = self.simplify_args();
    let lhs = sub_exprs[0].clone();
    let rhs = sub_exprs[1].clone();
    if lhs == rhs {
      self.id = Context::TRUE_ID;
    } else {
      *self = self.ctx.same_object(lhs, rhs)
    }
  }
}