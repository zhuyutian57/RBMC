
use std::collections::HashMap;

use crate::expr::expr::*;
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
    Symbol::new(ident, l1_num, 0, 1)
  }

  pub fn l2_symbol(&mut self, ident: NString) -> Symbol {
    let l1_num = self.l1_num(ident, false);
    let l1_name = self.l1_symbol(ident.clone()).l1_name();
    let l2_num = self.l2_num(l1_name, false);
    Symbol::new(ident, l1_num, l2_num, 2)
  }

  pub fn new_l1_symbol(&mut self, ident: NString) -> Symbol {
    let l1_num = self.l1_num(ident.clone(), true);
    Symbol::new(ident, l1_num, 0, 1)
  }

  pub fn new_l2_symbol(&mut self, ident: NString) -> Symbol {
    let l1_num = self.l1_num(ident.clone(), false);
    let l1_name = self.l1_symbol(ident.clone()).l1_name();
    let l2_num = self.l2_num(l1_name, true);
    Symbol::new(ident, l1_num, l2_num, 2)
  }

  pub fn constant_propagate(&mut self, symbol: Symbol, constant: Expr) {
    self
      .constant_map
      .entry(symbol)
      .and_modify(|x| *x = constant.clone())
      .or_insert(constant);
  }

  pub fn l1_rename(&mut self, expr: &mut Expr) {
    // Do more job
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
      
      return;
    }
  }

  pub fn l2_rename(&mut self, expr: &mut Expr) {
    todo!();
    // Do more job

    if expr.is_symbol() {
      let symbol = expr.symbol();
      if symbol.is_level2() { return; }
      if !symbol.is_level1() { self.l1_rename(expr); }
      let l1_symbol = expr.symbol();
      let l2_symbol = self.l2_symbol(l1_symbol.l1_name());

      if self.constant_map.contains_key(&l2_symbol) {
        *expr = self.constant_map.get(&l2_symbol).unwrap().clone();
      } else {
        *expr = expr.ctx.symbol(l2_symbol, expr.ty());
      }
      return;
    }
  }
}


