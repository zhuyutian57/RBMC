use std::collections::HashMap;

use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::symbol::nstring::NString;

/// Use for record `l0` symbol, `l0` global objects and counting
/// the number of non-deterministic variable for each type.
#[derive(Debug, Default)]
pub(crate) struct Namespace {
    _nondet_counting: HashMap<Type, usize>,
    objects: HashMap<NString, Expr>,
    symbols: HashMap<NString, Expr>,
}

impl Namespace {
    pub fn containts_symbol(&mut self, ident: NString) -> bool {
        self.symbols.contains_key(&ident)
    }

    pub fn insert_object(&mut self, expr: Expr) {
        assert!(expr.is_object());
        let inner = expr.extract_inner_expr();
        assert!(inner.is_symbol());
        let symbol = inner.extract_symbol();
        assert!(symbol.is_level0());
        assert!(!self.objects.contains_key(&symbol.ident()));
        self.objects.insert(symbol.ident(), expr);
    }

    pub fn insert_symbol(&mut self, expr: Expr) {
        assert!(expr.is_symbol());
        let symbol = expr.extract_symbol();
        assert!(symbol.is_level0());
        assert!(!self.symbols.contains_key(&symbol.ident()));
        self.symbols.insert(symbol.ident(), expr);
    }

    pub fn clear_local_symbols(&mut self, function_id: NString) {
        self.symbols.retain(|x, _| !x.contains(function_id));
    }

    pub fn lookup_nondet_count(&mut self, ty: Type) -> usize {
        self
            ._nondet_counting
            .entry(ty)
            .and_modify(|x| *x += 1)
            .or_insert(1)
            .clone()
    }

    pub fn lookup_symbol(&self, ident: NString) -> Expr {
        self.symbols.get(&ident).expect("Not exists").clone()
    }

    pub fn lookup_object(&self, ident: NString) -> Expr {
        self.objects.get(&ident).expect("Not exists").clone()
    }
}
