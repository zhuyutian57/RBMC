
use std::collections::HashMap;

use crate::expr::expr::*;
use crate::expr::op::*;
use crate::symbol::{symbol::*, nstring::*};

/// Renaming for symbol
/// 
/// l1_renaming: counting for locals
/// l2_renaming: counting for l1 symbol
/// constant_map: constant for l2 symbol
#[derive(Default)]
pub struct Renaming {
  l1_renaming: HashMap<NString, usize>,
  l2_renaming: HashMap<NString, usize>,
  constant_map: HashMap<Symbol, Expr>,
}

impl Renaming {
  fn l1_num(&mut self, ident: NString, inc: bool) -> usize {
    *self
      .l1_renaming
      .entry(ident)
      .and_modify(|x| if inc { *x += 1; })
      .or_insert(1)
  }

  fn l2_num(&mut self, ident: NString, inc: bool) -> usize {
    *self
      .l2_renaming
      .entry(ident)
      .and_modify(|x| if inc { *x += 1; })
      .or_insert(1)
  }

  pub fn l1_symbol(&mut self, ident: NString) -> Symbol {
    let l1_num = self.l1_num(ident, false);
    Symbol::new(ident, l1_num, 0, Level::level1)
  }

  pub fn l2_symbol(&mut self, ident: NString) -> Symbol {
    let l1_num = self.l1_num(ident, false);
    let l1_name = self.l1_symbol(ident.clone()).l1_name();
    let l2_num = self.l2_num(l1_name, false);
    Symbol::new(ident, l1_num, l2_num, Level::level2)
  }

  pub fn new_l1_symbol(&mut self, ident: NString) -> Symbol {
    let l1_num = self.l1_num(ident.clone(), true);
    Symbol::new(ident, l1_num, 0, Level::level1)
  }

  pub fn new_l2_symbol(&mut self, ident: NString) -> Symbol {
    let l1_num = self.l1_num(ident.clone(), false);
    let l1_name = self.l1_symbol(ident.clone()).l1_name();
    let l2_num = self.l2_num(l1_name, true);
    Symbol::new(ident, l1_num, l2_num, Level::level2)
  }

  pub fn constant_propagate(&mut self, mut lhs: Expr, constant: Expr) {
    self
      .constant_map
      .entry(lhs.symbol())
      .and_modify(|x| *x = constant.clone())
      .or_insert(constant);
  }

  fn rename_replace(&self, expr: &mut Expr, sub_exprs: Vec<Expr>) {
    let ctx = expr.ctx.clone();
    if expr.is_binary() {
      let lhs = sub_exprs[0].clone();
      let rhs = sub_exprs[1].clone();
      *expr =
        match expr.binOp() {
          BinOp::Add => ctx.add(lhs, rhs),
          BinOp::Sub => ctx.sub(lhs, rhs),
          BinOp::Mul => ctx.mul(lhs, rhs),
          BinOp::Div => ctx.div(lhs, rhs),
          BinOp::Eq => ctx.eq(lhs, rhs),
          BinOp::Ne => ctx.ne(lhs, rhs),
          BinOp::Ge => ctx.ge(lhs, rhs),
          BinOp::Gt => ctx.gt(lhs, rhs),
          BinOp::Le => ctx.le(lhs, rhs),
          BinOp::Lt => ctx.lt(lhs, rhs),
          BinOp::And => ctx.and(lhs, rhs),
          BinOp::Or => ctx.or(lhs, rhs),
        };
    }

    if expr.is_unary() {
      let operand = sub_exprs[0].clone();
      *expr =
        match expr.unOp() {
          UnOp::Not => ctx.not(operand),
          UnOp::Neg => ctx.neg(operand),
        }
    }

    if expr.is_object() {
      let o = sub_exprs[0].clone();
      *expr = ctx.object(o);
    }
  }

  pub fn l1_rename(&mut self, expr: &mut Expr) {
    if expr.is_terminal() {
      if expr.is_symbol() {
        let mut symbol = expr.symbol();
        if !symbol.is_level1() {
          symbol = self.l1_symbol(symbol.identifier());
        }
        
        if self.constant_map.contains_key(&symbol) {
          *expr = self.constant_map.get(&symbol).unwrap().clone();
        } else {
          *expr = expr.ctx.symbol(symbol, expr.ty());
        }
      }
      return;
    }

    // Expr is not a leaf. There must be some sub-nodes in AST
    let mut sub_exprs = expr.sub_exprs().unwrap();
    for sub_expr in sub_exprs.iter_mut() { self.l1_rename(sub_expr); }

    self.rename_replace(expr, sub_exprs);
  }

  pub fn l2_rename(&mut self, expr: &mut Expr) {
    if expr.is_terminal() {
        if expr.is_symbol() {
        let symbol = expr.symbol();

        assert!(symbol.is_level1());
        let l1_symbol = expr.symbol();
        let l2_symbol = self.l2_symbol(l1_symbol.l1_name());

        if self.constant_map.contains_key(&l2_symbol) {
          *expr = self.constant_map.get(&l2_symbol).unwrap().clone();
        } else {
          *expr = expr.ctx.symbol(l2_symbol, expr.ty());
        }
      }
      return;
    }

    // Expr is not a leaf. There must be some sub-nodes in AST
    let mut sub_exprs = expr.sub_exprs().unwrap();
    for sub_expr in sub_exprs.iter_mut() { self.l2_rename(sub_expr); }

    self.rename_replace(expr, sub_exprs);
  }
}


