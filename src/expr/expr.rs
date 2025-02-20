
use std::fmt::Debug;
use std::fmt::Error;
use std::hash::Hash;

use crate::symbol::symbol::*;

use super::ast::*;
use super::constant::*;
use super::context::*;
use super::op::*;
use super::predicates::*;
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
  pub fn is_ite(&self) -> bool { self.ctx.borrow().is_ite(self.id) }
  pub fn is_cast(&self) -> bool { self.ctx.borrow().is_cast(self.id) }
  pub fn is_object(&self) -> bool { self.ctx.borrow().is_object(self.id) }
  pub fn is_same_object(&self) -> bool { self.ctx.borrow().is_same_object(self.id) }
  pub fn is_index(&self) -> bool { self.ctx.borrow().is_index(self.id) }
  pub fn is_store(&self) -> bool { self.ctx.borrow().is_store(self.id) }

  pub fn extract_symbol(&self) -> Symbol {
    self
      .ctx
      .borrow()
      .extract_symbol(self.id)
      .expect("Not symbol")
  }

  pub fn extract_constant(&self) -> Constant {
    self
      .ctx
      .borrow()
      .extract_constant(self.id)
      .expect("Not constant")
  }

  pub fn extract_integer(&self) -> BigInt {
    self.extract_constant().to_integer()
  }

  pub fn extract_layout(&self) -> Type {
    self
      .ctx
      .borrow()
      .extract_type(self.id)
      .expect("Not layout")
  }

  pub fn extract_object(&self) -> Expr {
    assert!(
      self.is_address_of() ||
      self.is_index() ||
      self.is_store()
    );
    self.sub_exprs().unwrap().remove(0)
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
    self.sub_exprs().unwrap().remove(0)
  }
  
  pub fn extract_target_type(&self) -> Type {
    assert!(self.is_cast());
    self.sub_exprs().unwrap().remove(1).extract_layout()
  }

  pub fn extract_inner_expr(&self) -> Expr {
    assert!(self.is_object());
    self.sub_exprs().unwrap().remove(0)
  }

  pub fn extract_index(&self) -> Expr {
    assert!(self.is_index() || self.is_store());
    self.sub_exprs().unwrap().remove(1)
  }

  pub fn extract_update_value(&self) -> Expr {
    assert!(self.is_store());
    self.sub_exprs().unwrap().remove(2)
  }

  pub fn extract_ownership(&self) -> Ownership {
    if self.is_object() {
      self.ctx.borrow().extract_ownership(self.id).unwrap()
    } else if self.is_index() {
      self.extract_object().extract_ownership()
    } else {
      panic!("Do not have ownership:\n{self:?}")
    }
  }

  pub fn simplify(&mut self) {
    if let Some(mut sub_exprs) = self.sub_exprs() {
      for sub_expr in sub_exprs.iter_mut() { sub_expr.simplify(); }
      if self.is_binary() {
        let lhs = &sub_exprs[0];
        let rhs = &sub_exprs[1];
        match self.extract_bin_op() {
          BinOp::And => {
            if lhs.is_true() {
              self.id = rhs.id;
            } else if rhs.is_true() {
              self.id = lhs.id;
            } else if lhs.is_false() || rhs.is_false() {
              self.id = Context::FALSE_ID;
            } else {
              *self = self.ctx.and(lhs.clone(), rhs.clone());
            }
          },
          BinOp::Or => {
            if lhs.is_false() {
              self.id = rhs.id;
            } else if rhs.is_false() {
              self.id = lhs.id;
            } else if lhs.is_true() || rhs.is_true() {
              self.id = Context::TRUE_ID;
            } else {
              *self = self.ctx.or(lhs.clone(), rhs.clone());
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
      write!(
        f, "{:?}",
        self.ctx.borrow().extract_terminal(self.id).unwrap()
      )
    } else {
      let sub_exprs = self.sub_exprs().unwrap();

      if self.is_address_of() {
        let place = &sub_exprs[0];
        return write!(f, "&{place:?}");
      }
      
      if self.is_binary() {
        let lhs = &sub_exprs[0];
        let rhs = &sub_exprs[1];
        return write!(f, "{lhs:?} {:?} {rhs:?}", self.extract_bin_op());
      }

      if self.is_unary() {
        return write!(f, "{:?}{:?}", self.extract_un_op(), sub_exprs[0]);
      }

      if self.is_cast() {
        let lhs = &sub_exprs[0];
        let ty = &sub_exprs[1];
        return write!(f, "{lhs:?} as {ty:?}");
      }

      if self.is_object() {
        return write!(f, "{:?}", sub_exprs[0]);
      }

      if self.is_index() {
        let object = &sub_exprs[0];
        let index = &sub_exprs[1];
        return write!(f, "{object:?}.{index:?}");
      }

      if self.is_ite() {
        let cond = &sub_exprs[0];
        let true_value = &sub_exprs[1];
        let false_value = &sub_exprs[2];
        return write!(f, "{:?} ? {:?} : {:?}", cond, true_value, false_value);
      }

      if self.is_same_object() {
        let lhs = &sub_exprs[0];
        let rhs = &sub_exprs[1];
        return write!(f, "same_object({lhs:?}, {rhs:?})");
      }

      if self.is_store() {
        let object = &sub_exprs[0];
        let index = &sub_exprs[1];
        let value = &sub_exprs[2];
        return write!(f, "store({object:?}, {index:?}, {value:?})");
      }

      println!("Incomplete Debug for Expr");
      Err(Error)
    }
  }
}

pub trait ExprBuilder {
  fn constant_bool(&self, b: bool) -> Expr;
  fn constant_integer(&self, i: BigInt, ty: Type) -> Expr;
  fn constant_array(&self, constant: Constant, elem_ty: Type) -> Expr;
  fn constant_struct(&self, fields: Vec<StructField>, ty: Type) -> Expr;
  fn mk_symbol(&self, symbol: Symbol, ty: Type) -> Expr;
  fn mk_type(&self, ty: Type) -> Expr;

  fn address_of(&self, object: Expr, ty: Type) -> Expr;

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
  fn ite(&self, cond: Expr, true_value: Expr, false_value: Expr) -> Expr;
  fn cast(&self, operand: Expr, target_ty: Expr) -> Expr;

  fn object(&self, object: Expr, ownership: Ownership) -> Expr;
  fn same_object(&self, lhs: Expr, rhs: Expr) -> Expr;
  fn index(&self, object: Expr, index: Expr, ty: Type) -> Expr;
  fn store(&self, object: Expr, key: Expr, value: Expr) -> Expr;
}