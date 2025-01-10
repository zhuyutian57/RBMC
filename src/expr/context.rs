use std::fmt::Debug;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::symbol::{nstring::*, symbol::*};
use super::{ast::*, constant::*, expr::*, op::*, ty::*};

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
    self.add_node(Node::new(NodeKind::Terminal(0), bool_type));
    
    self.add_terminal(Terminal::Constant(Constant::Bool(false)));
    self.add_node(Node::new(NodeKind::Terminal(1), bool_type));
    
    // Maybe more
  }

  pub fn ty(&self, i: NodeId) -> Type {
    assert!(i < self.nodes.len());
    self.nodes[i].ty()
  }

  fn add_node(&mut self, node: Node) -> NodeId {
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

  fn add_terminal(&mut self, terminal: Terminal) -> TerminalId {
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

  pub fn is_terminal(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Terminal(_))
  }

  pub fn is_true(&self, i: NodeId) -> bool { i == 0 }

  pub fn is_false(&self, i: NodeId) -> bool { i == 1 }

  pub fn is_constant(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(
      self.terminal(i),
      Ok(t) if matches!(*t, Terminal::Constant(_))
    )
  }

  pub fn is_layout(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(
      self.terminal(i),
      Ok(t) if matches!(*t, Terminal::Layout(_))
    )
  }

  pub fn is_symbol(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(
      self.terminal(i),
      Ok(t) if matches!(*t, Terminal::Symbol(_))
    )
  }

  pub fn is_address_of(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::AddressOf(_))
  }

  pub fn is_binary(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Binary(_, _, _))
  }

  pub fn is_unary(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Unary(_, _))
  }

  pub fn is_object(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.nodes[i].kind(), NodeKind::Object(_))
  }

  pub fn terminal(&self, i: NodeId) -> Result<Rc<Terminal>, &str> {
    assert!(i < self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Terminal(t)
        => Ok(self.terminals[t].clone()),
      _ => Err("Not terminal"),
    }
  }
  
  pub fn symbol(&self, i: NodeId) -> Result<Symbol, &str> {
    assert!(i < self.nodes.len());
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
    assert!(i < self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Terminal(t) => {
        match &*self.terminals[t] {
          Terminal::Layout(l) => Ok(l.clone()),
          _ => Err("Not layout"),
        }
      }
      _ => Err("Not layout"),
    }
  }

  pub fn bin_op(&self, i: NodeId) -> Result<BinOp, &str> {
    assert!(self.is_binary(i));
    match self.nodes[i].kind() {
      NodeKind::Binary(op, _, _) => Ok(op),
      _ => Err("Not binary operator"),
    }
  }

  pub fn un_op(&self, i: NodeId) ->  Result<UnOp, &str> {
    assert!(self.is_unary(i));
    match self.nodes[i].kind() {
      NodeKind::Unary(op, _,) => Ok(op),
      _ => Err("Not unary operator"),
    }
  }

  pub fn sub_nodes(&self, i: NodeId) -> Option<Vec<NodeId>> {
    assert!(i < self.nodes.len());
    self.nodes[i].sub_nodes()
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

  fn constant_integer(&self, sign: bool, value: u128, ty: Type) -> Expr {
    let terminal = Terminal::Constant(Constant::Integer(sign, value));
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn constant_struct(&self, constants: Vec<Constant>, ty: Type) -> Expr {
    let terminal = Terminal::Constant(Constant::Struct(constants));
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn symbol(&self, symbol: Symbol, ty: Type) -> Expr {
    let terminal = Terminal::Symbol(symbol);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn layout(&self, ty: Type) -> Expr {
    let terminal = Terminal::Layout(ty);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn address_of(&self, place: Expr, ty: Type) -> Expr {
    let kind = NodeKind::AddressOf(place.id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn add(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Add, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn sub(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Sub, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn mul(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Mul, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn div(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Div, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn eq(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Eq, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn ne(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Ne, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn ge(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Ge, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn gt(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Gt, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn le(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Le, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn lt(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Lt, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn and(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    assert!(lhs.ty().is_bool());
    let kind = NodeKind::Binary(BinOp::And, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn or(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    assert!(lhs.ty().is_bool());
    let kind = NodeKind::Binary(BinOp::Or, lhs.id, rhs.id);
    let ty = lhs.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn not(&self, operand: Expr) -> Expr {
    assert!(operand.ty().is_bool());
    let kind = NodeKind::Unary(UnOp::Not, operand.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn neg(&self, operand: Expr) -> Expr {
    assert!(operand.ty().is_signed());
    let kind = NodeKind::Unary(UnOp::Neg, operand.id);
    let ty = operand.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
    
  }

  fn object(&self, object: Expr) -> Expr {
    assert!(object.is_terminal()); // TODO: other expr
    let kind = NodeKind::Object(object.id);
    let ty = object.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }
  
  fn ite(&self, cond: Expr, true_value: Expr, false_value: Expr) -> Expr {
    assert!(cond.ty().is_bool());
    assert!(true_value.ty() == false_value.ty());
    let kind = NodeKind::Ite(cond.id, true_value.id, false_value.id);
    let ty = true_value.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }
  
  fn same_object(&self, lhs: Expr, rhs: Expr) -> Expr {
    let kind = NodeKind::SameObject(lhs.id, rhs.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }
}