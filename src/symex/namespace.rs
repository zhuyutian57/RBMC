
use std::collections::HashMap;

use crate::NString;
use crate::expr::expr::*;

/// Use for record `l0` symbol expr
#[derive(Debug, Default)]
pub(crate) struct Namespace {
  table: HashMap<NString, Expr>,
}

impl Namespace {
  pub fn containts(&mut self, ident: NString) -> bool {
    self.table.contains_key(&ident)
  }

  pub fn insert(&mut self, ident: Expr) {
    assert!(ident.is_symbol());
    let symbol = ident.extract_symbol();
    assert!(symbol.is_level0());
    assert!(!self.table.contains_key(&symbol.ident()));
    self.table.insert(symbol.ident(), ident);
  }

  pub fn remove(&mut self, name: NString) {
    self.table.remove(&name);
  }

  pub fn lookup(&self, ident: NString) -> Expr {
    self
      .table
      .get(&ident)
      .expect("Not exists")
      .clone()
  }
}