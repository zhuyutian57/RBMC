use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::BitAndAssign;
use std::ops::BitOrAssign;
use std::ops::Sub;

use crate::expr::op::BinOp;

use super::context::*;
use super::expr::*;

/// Guard is an special expr that in CNF form.
#[derive(Clone)]
pub struct Guard {
    _ctx: ExprCtx,
    _expr_set: HashSet<Expr>,
}

impl Guard {
    pub fn new(_ctx: ExprCtx) -> Self {
        let mut _expr_set = HashSet::new();
        _expr_set.insert(_ctx._true());
        Guard { _ctx, _expr_set }
    }

    pub fn make_true(&mut self) {
        self._expr_set.clear();
        self._expr_set.insert(self._ctx._true());
    }

    pub fn make_false(&mut self) {
        self._expr_set.clear();
        self._expr_set.insert(self._ctx._false());
    }

    pub fn is_true(&self) -> bool {
        self._expr_set.len() == 1 && self._expr_set.contains(&self._ctx._true())
    }

    pub fn is_false(&self) -> bool {
        self._expr_set.len() == 1 && self._expr_set.contains(&self._ctx._false())
    }

    pub fn add(&mut self, mut expr: Expr) {
        assert!(expr.ty().is_bool());
        expr.simplify();
        if self.is_false() || expr.is_true() {
            return;
        }
        if expr.is_false() {
            self.make_false();
            return;
        }

        if expr.is_binary() && expr.extract_bin_op() == BinOp::And {
            let sub_exprs = expr.sub_exprs();
            self.add(sub_exprs[0].clone());
            self.add(sub_exprs[1].clone());
        } else {
            let mut not_expr = self._ctx.not(expr.clone());
            not_expr.simplify();
            if self._expr_set.contains(&not_expr) {
                self.make_false();
            } else {
                self._expr_set.insert(expr);
            }
        }
    }

    pub fn guard(&self, conseq: Expr) -> Expr {
        self._ctx.implies(self.to_expr(), conseq)
    }

    pub fn to_expr(&self) -> Expr {
        let mut res =
            self._expr_set.iter().fold(self._ctx._true(), |acc, x| self._ctx.and(acc, x.clone()));
        res.simplify();
        res
    }
}

impl Sub for Guard {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        let _expr_set =
            self._expr_set.difference(&rhs._expr_set).map(|x| x.clone()).collect::<HashSet<_>>();
        Guard { _ctx: self._ctx, _expr_set }
    }
}

impl BitAndAssign<&Guard> for Guard {
    fn bitand_assign(&mut self, rhs: &Guard) {
        for expr in rhs._expr_set.iter() {
            self.add(expr.clone());
        }
    }
}

impl BitOrAssign<&Guard> for Guard {
    fn bitor_assign(&mut self, rhs: &Guard) {
        if self.is_true() || rhs.is_false() {
            return;
        }
        if self.is_false() || rhs.is_true() {
            *self = rhs.clone();
            return;
        }

        if self._expr_set.len() == 1 && rhs._expr_set.len() == 1 {
            let g1 = self.to_expr();
            let g2 = rhs.to_expr();
            self._expr_set.clear();
            self.add(self._ctx.or(g1, g2));
        } else {
            // Common
            let common = self._expr_set
                .intersection(&rhs._expr_set)
                .map(|x| x.clone())
                .collect::<HashSet<_>>();
            let g1 = self._expr_set
                .difference(&common)
                .map(|x| x.clone())
                .fold(self._ctx._true(), |acc, x| self._ctx.and(acc, x));
            let g2 = rhs._expr_set
                .difference(&common)
                .map(|x| x.clone())
                .fold(self._ctx._true(), |acc, x| self._ctx.and(acc, x));
            self._expr_set = common;
            let mut new_g = self._ctx.or(g1, g2);
            new_g.simplify();
            self._expr_set.insert(new_g);
        }
    }
}

impl From<Expr> for Guard {
    fn from(value: Expr) -> Self {
        let mut guard = Guard::new(value.ctx.clone());
        guard.add(value);
        guard
    }
}

impl Debug for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_expr())
    }
}
