use std::fmt::{Debug, Error};
use std::ops::Deref;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use stable_mir::mir::*;
use stable_mir::ty::*;

use crate::nstring::NString;
use crate::program::{self, *};

use super::ast::*;
use super::constant::*;
use super::expr::*;
use super::symbol::*;
use super::ty::*;

type BinOp = super::ast::BinOp;
type UnOp = super::ast::UnOp;

/// Context is used to manage type and expression.
/// TODO: do memory management
#[derive(Default)]
pub struct Context {
  nodes: Vec<Node>,
  node_map: HashMap<Node, NodeId>,
  terminals: Vec<Rc<Terminal>>,
  terminal_map: HashMap<NString, TerminalId>,
}

impl Context {
  pub fn new() -> Self {
    let mut ctx = Context::default();
    ctx.init_terminals();
    ctx
  }

  fn init_terminals(&mut self) {
    let bool_type = Type::bool_type();

    self.add_terminal(Terminal::Constant(Constant::Bool(true)));
    self.add_node(Node::terminal(0, bool_type));
    
    self.add_terminal(Terminal::Constant(Constant::Bool(false)));
    self.add_node(Node::terminal(1, bool_type));
    
    // Maybe more
  }

  pub fn node(&self, i: NodeId) -> Node {
    assert!(i <= self.nodes.len());
    self.nodes[i].clone()
  }

  pub fn node_ty(&self, i: NodeId) -> Type {
    assert!(i <= self.nodes.len());
    self.nodes[i].ty()
  }

  pub fn add_node(&mut self, node: Node) -> NodeId {
    if self.node_map.contains_key(&node) {
      *(self.node_map.get(&node).unwrap())
    } else {
      self.nodes.push(node.clone());
      self
        .node_map
        .insert(node.clone(), self.nodes.len() - 1);
      self.nodes.len() - 1
    }
  }

  pub fn terminal(&self, i: TerminalId) -> Rc<Terminal> {
    assert!(i <= self.terminals.len());
    self.terminals[i].clone()
  }

  pub fn add_terminal(&mut self, terminal: Terminal) -> TerminalId {
    let ident = terminal.identifier();
    if self.terminal_map.contains_key(&ident) {
      *(self.terminal_map.get(&ident).unwrap())
    } else {
      self.terminals.push(Rc::new(terminal));
      let id = self.terminals.len() - 1;
      self
        .terminal_map
        .insert(ident, id);
      id
    }
  }

  pub fn is_true(&self, i: NodeId) -> bool { i == 0 }

  pub fn is_false(&self, i: NodeId) -> bool { i == 1 }

  pub fn is_binary(&self, i: NodeId) -> bool {
    assert!(i <= self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Binary(_, _, _))
  }

  pub fn is_unary(&self, i: NodeId) -> bool {
    assert!(i <= self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Unary(_, _))
  }

  pub fn is_object(&self, i: NodeId) -> bool {
    assert!(i <= self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Object(_))
  }

  pub fn is_terminal(&self, i: NodeId) -> bool {
    assert!(i <= self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Terminal(_))
  }

  pub fn is_symbol(&self, i: NodeId) -> bool {
    assert!(i <= self.nodes.len());
    matches!(
      self.nodes[i].kind(),
      NodeKind::Terminal(t)
        if matches!(*self.terminal(t), Terminal::Symbol(_))
    )
  }

  pub fn is_layout(&self, i: NodeId) -> bool {
    assert!(i <= self.nodes.len());
    matches!(
      self.nodes[i].kind(),
      NodeKind::Terminal(t)
        if matches!(*self.terminal(t), Terminal::Layout(_))
    )
  }

  pub fn symbol(&self, i: NodeId) -> Result<Symbol, &str> {
    assert!(i <= self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Terminal(t) => {
        match &*self.terminals[t] {
          Terminal::Symbol(s) => Ok(s.clone()),
          _ => Err("Not symbol"),
        }
      },
      _ => Err("Not symbol")
    }
  }

  pub fn layout(&self, i: NodeId) -> Result<Type, &str> {
    assert!(i <= self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Terminal(t) => {
        match &*self.terminals[t] {
          Terminal::Layout(l) => Ok(l.clone()),
          _ => Err("Not symbol"),
        }
      }
      _ => Err("Not layout"),
    }
  }

}

impl Debug for Context {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut nodes = String::from("Nodes:\n");
    let mut i = 0;
    for node in &self.nodes {
      nodes.push_str(format!("{i} -> {node:?}\n").as_str());
      i += 1;
    }
    nodes.push_str(format!("{:?}\n", self.node_map).as_str());
    let mut terminals = String::from("Terminals:\n");
    i = 0;
    for terminal in &self.terminals {
      terminals.push_str(format!("{i} -> {terminal:?}\n").as_str());
      i += 1;
    }
    terminals.push_str(format!("{:?}\n", self.terminal_map).as_str());
    write!(f, "{}{}", nodes, terminals)
  }
}

pub type ExprCtx = Rc<RefCell<Context>>;

impl ExprBuilder for ExprCtx {
  fn constant_bool(&self, b: bool) -> Expr {
    Expr { ctx: self.clone(), id: if b { 0 } else { 1 } }
  }

  fn symbol(&self, symbol: Symbol, ty: Type) -> Expr {
    let terminal = Terminal::Symbol(symbol);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let new_node = Node::terminal(terminal_id, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn layout(&self, ty: Type) -> Expr {
    let terminal = Terminal::Layout(ty);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let new_node = Node::terminal(terminal_id, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn add(&self, lhs: Expr, rhs: Expr) -> Expr {
    todo!()
  }

  fn sub(&self, lhs: Expr, rhs: Expr) -> Expr {
    todo!()
  }

  fn mul(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn div(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn eq(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn ne(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn ge(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn gt(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn le(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn lt(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn and(&self, lhs: Expr, rhs: Expr) -> Expr {
      todo!()
  }

  fn or(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(self.as_ptr() == lhs.ctx.as_ptr());
    assert!(lhs.ctx.as_ptr() == rhs.ctx.as_ptr());
    assert!(lhs.ty().is_bool());
    assert!(lhs.ty() == rhs.ty());
    let new_node =
      Node::binary(BinOp::Or, lhs.id, rhs.id, lhs.ty());
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn not(&self, operand: Expr) -> Expr {
      todo!()
  }

  fn neq(&self, operand: Expr) -> Expr {
      todo!()
  }

  fn object(&self, obj: Expr) -> Expr {
    assert!(obj.is_terminal()); // TODO: other expr
    let new_node = Node::object(obj.id, obj.ty());
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }
}