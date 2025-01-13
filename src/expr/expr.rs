
use std::fmt::{Debug, Error};
use std::hash::Hash;

use crate::symbol::symbol::*;

use super::ast::*;
use super::constant::*;
use super::context::*;
use super::op::*;
use super::ty::*;

/// `Expr` is a wrapper for AST node. It only carry node index that
/// is used to construct AST. The corresponding information can be
/// retrieved from `Context` 
#[derive(Clone)]
pub struct Expr {
  pub ctx: ExprCtx,
  pub(super) id: NodeId,
}

impl Expr {
  pub fn ty(&self) -> Type { self.ctx.borrow().ty(self.id) }

  pub fn is_terminal(&self) -> bool { self.ctx.borrow().is_terminal(self.id) }
  pub fn is_true(&self) -> bool { self.ctx.borrow().is_true(self.id) }
  pub fn is_false(&self) -> bool { self.ctx.borrow().is_false(self.id) }
  pub fn is_constant(&self) -> bool { self.ctx.borrow().is_constant(self.id) }
  pub fn is_symbol(&self) -> bool { self.ctx.borrow().is_symbol(self.id) }
  pub fn is_type(&self) -> bool { self.ctx.borrow().is_type(self.id) }

  pub fn is_address_of(&self) -> bool { self.ctx.borrow().is_address_of(self.id) }
  pub fn is_binary(&self) -> bool { self.ctx.borrow().is_binary(self.id) }
  pub fn is_unary(&self) -> bool { self.ctx.borrow().is_unary(self.id) }
  pub fn is_cast(&self) -> bool { self.ctx.borrow().is_cast(self.id) }
  pub fn is_object(&self) -> bool { self.ctx.borrow().is_object(self.id) }
  pub fn is_index_of(&self) -> bool { self.ctx.borrow().is_index_of(self.id) }
  pub fn is_ite(&self) -> bool { self.ctx.borrow().is_ite(self.id) }
  pub fn is_same_object(&self) -> bool { self.ctx.borrow().is_same_object(self.id) }

  pub fn extract_object(&self) -> Expr {
    assert!(self.is_address_of());
    self
      .sub_exprs()
      .expect("Wrong address_of")[0]
      .clone()
  }

  pub fn extract_bin_op(&self) -> BinOp {
    assert!(self.is_binary());
    self.ctx.borrow().extract_bin_op(self.id).unwrap()
  }

  pub fn extract_un_op(&self) -> UnOp {
    assert!(self.is_unary());
    self.ctx.borrow().extract_un_op(self.id).unwrap()
  }

  pub fn extract_src(&self) -> Expr {
    assert!(self.is_cast());
    self.sub_exprs().unwrap()[0].clone()
  }
  
  pub fn extract_target_type(&self) -> Type {
    assert!(self.is_cast());
    let ty = self.sub_exprs().unwrap()[1].clone();
    ty.extract_type()
  }

  pub fn extract_symbol(&self) -> Symbol {
    self
      .ctx
      .borrow()
      .extract_symbol(self.id)
      .expect("Not symbol")
  }

  pub fn extract_type(&self) -> Type {
    self
      .ctx
      .borrow()
      .extract_type(self.id)
      .expect("Not layout")
  }

  pub fn simplify(&mut self) {
    if let Some(mut sub_exprs) = self.sub_exprs() {
      for sub_expr in sub_exprs.iter_mut() { sub_expr.simplify(); }
      if self.is_binary() {
        let lhs = &sub_exprs[0];
        let rhs = &sub_exprs[1];
        match self.extract_bin_op() {
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
      }
      // TODO: do more simplify
    }
  }

  /// Construct sub-exprs from AST
  pub fn sub_exprs(&self) -> Option<Vec<Expr>>{
    match self.ctx.borrow().sub_nodes(self.id) {
      Some(ids) => {
        let mut sub_exprs = Vec::new();
        for id in ids {
          sub_exprs.push(Expr { ctx: self.ctx.clone(), id });
        }
        Some(sub_exprs)
      },
      None => None,
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
    if self.is_terminal() {
      write!(f, "{:?}", self.ctx.borrow().extract_terminal(self.id).unwrap())
    } else {
      let sub_exprs = self.sub_exprs().unwrap();

      if self.is_address_of() {
        let place = &sub_exprs[0];
        return write!(f, "&{place:?}");
      }
      
      if self.is_binary() {
        let lhs = &sub_exprs[0];
        let rhs = &sub_exprs[1];
        return write!(f, "{:?} {:?} {:?}", lhs, self.extract_bin_op(), rhs);
      }

      if self.is_unary() {
        return write!(f, "{:?} {:?}", self.extract_un_op(), sub_exprs[0]);
      }

      if self.is_cast() {
        let lhs = &sub_exprs[0];
        let ty = &sub_exprs[1];
        return write!(f, "{lhs:?} as {ty:?}");
      }

      if self.is_object() {
        return write!(f, "{:?}", sub_exprs[0]);
      }

      if self.is_index_of() {
        let object = &sub_exprs[0];
        let index = &sub_exprs[1];
        return write!(f, "{object:?}.{index:?}");
      }

      if self.is_ite() {
        let cond = &sub_exprs[0];
        let true_value = &sub_exprs[1];
        let false_value = &sub_exprs[2];
        return write!(f, "ite({:?}, {:?}, {:?})", cond, true_value, false_value);
      }

      if self.is_same_object() {
        let lhs = &sub_exprs[0];
        let rhs = &sub_exprs[1];
        return write!(f, "same_object({:?}, {:?})", lhs, rhs);
      }

      println!("Incomplete Debug for Expr");
      Err(Error)
    }
  }
}

pub trait ExprBuilder {
  fn constant_bool(&self, b: bool) -> Expr;
  fn constant_integer(&self, sign: bool, value: u128, ty: Type) -> Expr;
  fn constant_struct(&self, fields: Vec<Constant>, ty: Type) -> Expr;
  fn mk_symbol(&self, symbol: Symbol, ty: Type) -> Expr;
  fn mk_type(&self, ty: Type) -> Expr;

  fn address_of(&self, place: Expr, ty: Type) -> Expr;

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
  fn neg(&self, operand: Expr) -> Expr;
  fn cast(&self, operand: Expr, target_ty: Expr) -> Expr;
  fn object(&self, object: Expr) -> Expr;
  fn index_of(&self, object: Expr, index: usize, ty: Type) -> Expr;
  fn ite(&self, cond: Expr, true_value: Expr, false_value: Expr) -> Expr;
  fn same_object(&self, lhs: Expr, rhs: Expr) -> Expr;
}