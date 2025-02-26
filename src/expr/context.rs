
use std::cell::RefCell;
use std:: collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

use stable_mir::ty::*;

use crate::symbol::nstring::*;
use crate::symbol::symbol::*;
use super::ast::*;
use super::constant::*;
use super::expr::*;
use super::op::*;
use super::predicates::*;
use super::ty::*;

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
  pub const TRUE_ID : usize = 0;
  pub const FALSE_ID : usize = 1;

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
    self.nodes[i].kind().is_terminal()
  }

  pub fn is_true(&self, i: NodeId) -> bool { i == Context::TRUE_ID }

  pub fn is_false(&self, i: NodeId) -> bool { i == Context::FALSE_ID }

  pub fn is_constant(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(
      self.extract_terminal(i),
      Ok(t) if matches!(*t, Terminal::Constant(_))
    )
  }

  pub fn is_constant_bool(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.extract_constant(i), Ok(Constant::Bool(..)))
  }

  pub fn is_constant_integer(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.extract_constant(i), Ok(Constant::Integer(..)))
  }

  pub fn is_null(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.extract_constant(i), Ok(Constant::Null))
  }

  pub fn is_constant_array(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.extract_constant(i), Ok(Constant::Array(..)))
  }

  pub fn is_constant_struct(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(self.extract_constant(i), Ok(Constant::Struct(..)))
  }

  pub fn is_type(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(
      self.extract_terminal(i),
      Ok(t) if matches!(*t, Terminal::Type(_))
    )
  }

  pub fn is_symbol(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    matches!(
      self.extract_terminal(i),
      Ok(t) if matches!(*t, Terminal::Symbol(_))
    )
  }

  pub fn is_address_of(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_address_of()
  }

  pub fn is_binary(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_binary()
  }

  pub fn is_unary(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_unary()
  }

  pub fn is_ite(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_ite()
  }

  pub fn is_cast(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_cast()
  }

  pub fn is_object(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_object()
  }

  pub fn is_null_object(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    match self.extract_symbol(i) {
      Ok(s) => s.ident() == NString::NULL_OBJECT,
      _ => false,
    }
  }

  pub fn is_same_object(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_same_object()
  }

  pub fn is_index(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_index()
  }

  pub fn is_store(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_store()
  }

  pub fn is_pointer_ident(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_pointer_ident()
  }

  pub fn is_invalid(&self, i: NodeId) -> bool {
    assert!(i < self.nodes.len());
    self.nodes[i].kind().is_invalid()
  }

  pub(super) fn extract_terminal(&self, i: NodeId)
    -> Result<Rc<Terminal>, &str> {
    assert!(i < self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Terminal(t)
        => Ok(self.terminals[t].clone()),
      _ => Err("Not terminal"),
    }
  }
  
  pub fn extract_constant(&self, i: NodeId) -> Result<Constant, &str> {
    match self.is_constant(i) {
      true => Ok(self.extract_terminal(i).unwrap().to_constant()),
      false => Err("Not constant"),
    }
  }

  pub fn extract_type(&self, i: NodeId) -> Result<Type, &str> {
    match self.is_type(i) {
      true => Ok(self.extract_terminal(i).unwrap().to_type()),
      false => Err("Not type"),
    }
  }

  pub fn extract_symbol(&self, i: NodeId) -> Result<Symbol, &str> {
    match self.is_symbol(i) {
      true => Ok(self.extract_terminal(i).unwrap().to_symbol()),
      false => Err("Not symbol"),
    }
  }

  pub fn extract_bin_op(&self, i: NodeId) -> Result<BinOp, &str> {
    assert!(i < self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Binary(op, _, _) => Ok(op),
      _ => Err("Not binary operator"),
    }
  }

  pub fn extract_un_op(&self, i: NodeId) ->  Result<UnOp, &str> {
    assert!(i < self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Unary(op, _,) => Ok(op),
      _ => Err("Not unary operator"),
    }
  }

  pub fn extract_ownership(&self, i: NodeId) -> Result<Ownership, &str> {
    assert!(i < self.nodes.len());
    match self.nodes[i].kind() {
      NodeKind::Object(o, ..) => Ok(o),
      _ => Err("Not object"),
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

  fn constant_integer(&self, i: BigInt, ty: Type) -> Expr {
    let terminal = Terminal::Constant(Constant::Integer(i));
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn null(&self, ty: Type) -> Expr {
    let terminal = Terminal::Constant(Constant::Null);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, Type::null_type(ty));
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn null_mut(&self, ty: Type) -> Expr {
    let terminal = Terminal::Constant(Constant::Null);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, Type::null_mut_type(ty));
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn constant_array(&self, constant: Constant, elem_ty: Type) -> Expr {
    let terminal =
      Terminal::Constant(Constant::Array(Box::new(constant), elem_ty));
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let array_type = Type::const_array_type(elem_ty);
    let new_node = Node::new(kind, array_type);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn constant_struct(&self, fields: Vec<StructField>, ty: Type) -> Expr {
    let terminal = Terminal::Constant(Constant::Struct(fields));
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn mk_symbol(&self, symbol: Symbol, ty: Type) -> Expr {
    let terminal = Terminal::Symbol(symbol);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn mk_type(&self, ty: Type) -> Expr {
    let terminal = Terminal::Type(ty);
    let terminal_id = self.borrow_mut().add_terminal(terminal);
    let kind = NodeKind::Terminal(terminal_id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn address_of(&self, object: Expr, ty: Type) -> Expr {
    assert!(object.is_object());
    let kind = NodeKind::AddressOf(object.id);
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
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn ne(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Ne, lhs.id, rhs.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn ge(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Ge, lhs.id, rhs.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn gt(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Gt, lhs.id, rhs.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn le(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Le, lhs.id, rhs.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn lt(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty() == rhs.ty());
    let kind = NodeKind::Binary(BinOp::Lt, lhs.id, rhs.id);
    let ty = Type::bool_type();
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
  
  fn implies(&self, cond: Expr, conseq: Expr) -> Expr {
    assert!(cond.ty() == conseq.ty());
    assert!(cond.ty().is_bool());
    let kind = NodeKind::Binary(BinOp::Implies, cond.id, conseq.id);
    let ty = cond.ty();
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

  fn ite(&self, cond: Expr, true_value: Expr, false_value: Expr) -> Expr {
    assert!(cond.ty().is_bool());
    assert!(true_value.ty() == false_value.ty());
    let kind = NodeKind::Ite(cond.id, true_value.id, false_value.id);
    let ty = true_value.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn cast(&self, operand: Expr, target_ty: Expr) -> Expr {
    // TODO: check the compatibility
    let kind = NodeKind::Cast(operand.id, target_ty.id);
    let ty = target_ty.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn object(&self, object: Expr, ownership: Ownership) -> Expr {
    let kind = NodeKind::Object(ownership, object.id);
    let ty = object.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn null_object(&self, ty: Type) -> Expr {
    let symbol =
      Symbol::new(NString::NULL_OBJECT, 0, 0, Level::Level0);
    let symbol_expr = self.mk_symbol(symbol, ty);
    self.object(symbol_expr, Ownership::Not)
  }

  fn same_object(&self, lhs: Expr, rhs: Expr) -> Expr {
    assert!(lhs.ty().is_any_ptr() && rhs.ty().is_any_ptr());
    let kind = NodeKind::SameObject(lhs.id, rhs.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn index(&self, object: Expr, index: Expr, ty: Type) -> Expr {
    assert!(
      object.is_object() &&
      (object.ty().is_array() || object.ty().is_struct())
    );
    // TODO: match type
    let kind = NodeKind::Index(object.id, index.id);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn store(&self, object: Expr, key: Expr, value: Expr) -> Expr {
    let kind = NodeKind::Store(object.id, key.id, value.id);
    let ty = object.ty();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }
  
  fn pointer_ident(&self, pt: Expr) -> Expr {
    assert!(pt.ty().is_any_ptr());
    let kind = NodeKind::PointerIdent(pt.id);
    let ty = Type::unsigned_type(UintTy::Usize);
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }

  fn invalid(&self, object: Expr) -> Expr {
    assert!(object.is_object());
    let kind = NodeKind::Invalid(object.id);
    let ty = Type::bool_type();
    let new_node = Node::new(kind, ty);
    let id = self.borrow_mut().add_node(new_node);
    Expr { ctx: self.clone(), id }
  }
}