
use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Error};
use std::string::ToString;
use std::{alloc::Layout, any::type_name, hash::Hash};
use std::rc::Rc;
use std::cell::{RefCell, RefMut};

use super::ast::*;
use super::context::*;
use super::symbol::*;
use super::ty::Type;

/// `Expr` is a wrapper for AST node. It only carry node index that
/// is used to construct AST. The corresponding information can be
/// retrieved from `Context` 
#[derive(Clone)]
pub struct Expr {
  pub ctx: ExprCtx,
  pub(super) id: NodeId,
}

impl Expr {
  pub fn ty(&self) -> Type {
    self.ctx.borrow().node_ty(self.id)
  }

  pub fn is_true(&self) -> bool { self.ctx.borrow().is_true(self.id) }
  pub fn is_false(&self) -> bool { self.ctx.borrow().is_false(self.id) }
  pub fn is_binary(&self) -> bool { self.ctx.borrow().is_binary(self.id) }
  pub fn is_unary(&self) -> bool { self.ctx.borrow().is_unary(self.id) }
  pub fn is_object(&self) -> bool { self.ctx.borrow().is_object(self.id) }
  pub fn is_terminal(&self) -> bool { self.ctx.borrow().is_terminal(self.id) }

  pub fn is_symbol(&self) -> bool { self.ctx.borrow().is_symbol(self.id) }
  pub fn is_layout(&self) -> bool { self.ctx.borrow().is_layout(self.id) }

  pub fn symbol(&self) -> Symbol {
    self
      .ctx
      .borrow()
      .symbol(self.id)
      .expect("Not symbol")
  }

  pub fn layout(&self) -> Type {
    self
      .ctx
      .borrow()
      .layout(self.id)
      .expect("Not layout")
  }

  pub fn simplify(&mut self) {
    let node = self.ctx.borrow_mut().node(self.id);
    match node.kind() {
      NodeKind::Binary(op, l, r) => {
        let mut lhs = Expr { ctx: self.ctx.clone(), id: l};
        let mut rhs = Expr { ctx: self.ctx.clone(), id: r};
        lhs.simplify();
        rhs.simplify();
        match op {
          BinOp::And => {
            if lhs.is_true() && rhs.is_true() || lhs.is_false(){
              self.id = lhs.id
            } else if rhs.is_false() {
              self.id = rhs.id
            }
          },
          BinOp::Or => {
            if lhs.is_false() && rhs.is_false() || lhs.is_true() {
              self.id = lhs.id;
            } else if rhs.is_false() {
              self.id = rhs.id;
            }
          },
          _ => {},
        }
      },
      _ => {},
    }
  }
}

impl PartialEq for Expr {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}

impl Eq for Expr {}

impl Hash for Expr {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.id.hash(state);
  }
}

impl Debug for Expr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let node = self.ctx.borrow_mut().node(self.id);
    match node.kind() {
      NodeKind::Binary(op, l, r) => {
        let lhs = Expr { ctx: self.ctx.clone(), id: l};
        let rhs = Expr { ctx: self.ctx.clone(), id: r};
        f.write_fmt(format_args!("{:?} {:?} {:?}", lhs, op, rhs))
      },
      NodeKind::Unary(op, operand) => {
        let o = Expr { ctx: self.ctx.clone(), id: operand };
        f.write_fmt(format_args!("{:?} {:?}", op, o))
      },
      NodeKind::Terminal(t) => {
        f.write_fmt(format_args!("{:?}", self.ctx.borrow().terminal(t)))
      },
      NodeKind::Object(o) => {
        let node = self.ctx.borrow().node(o);
        let t = node.terminal_id().expect("Not terminal");
        f.write_fmt(format_args!("{:?}", self.ctx.borrow().terminal(t)))
      },
    }
  }
}

pub trait ExprBuilder {
  fn constant_bool(&self, b: bool) -> Expr;
  fn symbol(&self, symbol: Symbol, ty: Type) -> Expr;
  fn layout(&self, ty: Type) -> Expr;

  fn add(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn sub(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn mul(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn div(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn eq(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn ne(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn ge(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn gt(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn le(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn lt(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn and(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn or(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn not(&self, operand: Expr) -> Expr;
  fn neq(&self, operand: Expr) -> Expr;

  fn object(&self, obj: Expr) -> Expr;
}