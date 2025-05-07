use std::collections::HashMap;

use crate::expr::expr::*;
use crate::symbol::symbol::*;

/// Renaming for symbol
///
/// l1_renaming: counting for orgianl symbol
/// l2_renaming: counting for l1 symbol
/// constant_map: constant for l2 symbol
#[derive(Debug, Default, Clone)]
pub struct Renaming {
    l1_renaming: HashMap<Ident, usize>,
    l2_renaming: HashMap<(Ident, usize), usize>,
    constant_map: HashMap<Symbol, Expr>,
}

impl Renaming {
    fn l1_num(&mut self, key: Ident, inc: bool) -> usize {
        *self
            .l1_renaming
            .entry(key)
            .and_modify(|x| {
                if inc {
                    *x += 1;
                }
            })
            .or_insert(1)
    }

    fn l2_num(&mut self, key: (Ident, usize), inc: bool) -> usize {
        *self
            .l2_renaming
            .entry(key)
            .and_modify(|x| {
                if inc {
                    *x += 1;
                }
            })
            .or_insert(1)
    }

    pub fn variables(&self) -> Vec<Ident> {
        self.l1_renaming.keys().map(|&x| x).collect::<Vec<_>>()
    }

    pub fn l1_count(&self, key: Ident) -> usize {
        match self.l1_renaming.get(&key) {
            Some(n) => *n,
            None => 0,
        }
    }

    pub fn l2_count(&self, key: (Ident, usize)) -> usize {
        match self.l2_renaming.get(&key) {
            Some(n) => *n,
            None => 0,
        }
    }

    pub fn current_l1_symbol(&mut self, ident: Ident) -> Symbol {
        let l1_num = self.l1_num(ident, false);
        Symbol::new(ident, l1_num, 0, Level::Level1)
    }

    /// `l1_num = 0` means use the latest l1 number
    pub fn current_l2_symbol(&mut self, ident: Ident, mut l1_num: usize) -> Symbol {
        assert!(l1_num <= self.l1_num(ident, false));
        if l1_num == 0 {
            l1_num = self.l1_num(ident, false);
        }
        let l1_ident = (ident, l1_num);
        let l2_num = self.l2_num(l1_ident, false);
        Symbol::new(ident, l1_num, l2_num, Level::Level2)
    }

    pub fn new_l1_symbol(&mut self, ident: Ident) -> Symbol {
        let l1_num = self.l1_num(ident, true);
        Symbol::new(ident, l1_num, 0, Level::Level1)
    }

    /// `l1_num = 0` means use the latest l1 number
    pub fn new_l2_symbol(&mut self, ident: Ident, mut l1_num: usize) -> Symbol {
        assert!(l1_num <= self.l1_num(ident, false));
        if l1_num == 0 {
            l1_num = self.l1_num(ident, false);
        }
        let l1_ident = (ident, l1_num);
        let l2_num = self.l2_num(l1_ident, true);
        Symbol::new(ident, l1_num, l2_num, Level::Level2)
    }

    pub fn constant_propagate(&mut self, lhs: Expr, constant: Option<Expr>) {
        let symbol = lhs.extract_symbol();
        assert!(symbol.is_level1());
        if let Some(c) = constant {
            self.constant_map
                .entry(symbol)
                .and_modify(|x| *x = c.clone())
                .or_insert(c);
        } else {
            self.constant_map.remove(&symbol);
        };
    }

    pub fn l1_rename(&mut self, expr: &mut Expr) {
        if expr.is_terminal() {
            if expr.is_symbol() {
                let mut symbol = expr.extract_symbol();

                if symbol.is_level1() {
                    return;
                }

                symbol = self.current_l1_symbol(symbol.ident());
                *expr = expr.ctx.mk_symbol(symbol, expr.ty());
            }
            return;
        }

        // Expr is not a leaf. There must be some sub-nodes in AST
        let mut sub_exprs = expr.sub_exprs();
        for sub_expr in sub_exprs.iter_mut() {
            self.l1_rename(sub_expr);
        }

        expr.replace_sub_exprs(sub_exprs);
    }

    pub fn l2_rename(&mut self, expr: &mut Expr, propagate: bool) {
        if expr.is_address_of() {
            self.l1_rename(expr);
            return;
        }

        if expr.is_terminal() {
            if expr.is_symbol() {
                if expr.extract_symbol().is_level2() {
                    return;
                }

                self.l1_rename(expr);
                let symbol = expr.extract_symbol();

                if propagate && self.constant_map.contains_key(&symbol) {
                    *expr = self.constant_map.get(&symbol).unwrap().clone();
                } else {
                    let l2_symbol = self.current_l2_symbol(symbol.ident(), symbol.l1_num());
                    *expr = expr.ctx.mk_symbol(l2_symbol, expr.ty());
                }
            }
            return;
        }

        // Expr is not a leaf. There must be some sub-nodes in AST
        let mut sub_exprs = expr.sub_exprs();

        for (i, sub_expr) in sub_exprs.iter_mut().enumerate() {
            let prop = if i == 0 && expr.is_store() { false } else { propagate };
            self.l2_rename(sub_expr, prop);
        }

        expr.replace_sub_exprs(sub_exprs);
    }

    pub(super) fn remove_l1_renaming_by_key(&mut self, key: Ident) -> Option<usize> {
        self.l1_renaming.remove(&key)
    }

    pub(super) fn remove_l2_renaming_by_key(&mut self, key: (Ident, usize)) {
        self.l2_renaming.remove(&key);
    }

    pub(super) fn remove_constant_map_by_key(&mut self, key: Symbol) {
        self.constant_map.remove(&key);
    }
}
