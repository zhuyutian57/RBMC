
use std::collections::*;

use stable_mir::mir::*;

use crate::symbol::{symbol::*, nstring::*};
use crate::program::program::*;
use crate::expr::context::*;
use crate::expr::expr::*;
use super::state::*;
use super::renaming::*;

pub type Pc = BasicBlockIdx;

/// Each frame representing an execution of a function.
/// The id is used for naming variable. It is the unique
/// identifier for each frame.
pub struct Frame<'func> {
  ctx: ExprCtx,
  id: usize,
  function: &'func Function,
  /// Previous info. Used for recovering
  destination: Option<Place>,
  target: Option<BasicBlockIdx>,
  /// Current Computing
  pc: Pc,
  pub(super) cur_state: State,
  state_map: HashMap<Pc, Vec<State>>,
  renaming: Renaming,
}

impl<'func> Frame<'func> {
  pub fn new(
    ctx: ExprCtx,
    id: usize,
    function: &'func Function,
    destination: Option<Place>,
    target: Option<BasicBlockIdx>,
    state: State,
  ) -> Self {
    let mut state_map = HashMap::new();
    state_map.insert(0, vec![State::new(ctx.clone())]);
    Frame {
      ctx,
      id, function,
      destination,
      target,
      pc: 0,
      cur_state: state,
      state_map,
      renaming: Renaming::default(),
    }
  }

  pub fn destination(&self) -> &Option<Place> { &self.destination }

  pub fn target(&self) -> &Option<BasicBlockIdx> { &self.target }

  pub fn cur_pc(&self) -> Option<Pc> {
    if self.pc < self.function.size() {
      Some(self.pc)
    } else {
      None
    }
  }

  pub fn inc_pc(&mut self) {
    println!(
      "Done {:?} - bb{}\n{:?}",
      self.function.name(),
      self.pc,
      self.cur_state
    );
    self.pc += 1;
  }

  pub fn cur_state(&mut self) -> &mut State { &mut self.cur_state }

  pub fn add_state(&mut self, pc: Pc, state: State) {
    self
    .state_map
    .entry(pc)
    .or_default()
    .push(state);
  }

  pub fn states_from(&mut self, pc: Pc) -> Option<Vec<State>> {
    self.state_map.remove(&pc)
  }

  pub fn function(&self) -> &'func Function {
    self.function
  }

  fn local_ident(&self, local: Local) -> NString {
    self.function.name()
      + "_" + self.id.to_string()
      + "::" + local.to_string()
  }

  pub fn l1_local_count(&self, local: Local) -> usize {
    let ident = self.local_ident(local);
    self.renaming.count(ident, Level::level1)
  }

  pub fn l0_local(&self, local: Local) -> Expr {
    let symbol =
      Symbol::new(self.local_ident(local),0,0, Level::level0);
    let ty = self.function.local_decl(local).0;
    self.ctx.symbol(symbol, ty)
  }

  pub fn l1_local(&self, local: Local, mut l1_num: usize) -> Expr {
    if l1_num == 0 { l1_num = self.l1_local_count(local); }
    assert!(0 < l1_num && l1_num <= self.l1_local_count(local));
    let ident = self.local_ident(local);
    let symbol =
      Symbol::new(ident, l1_num, 0, Level::level1);
    let ty = self.function.local_decl(local).0;
    self.ctx.symbol(symbol, ty)
  }

  pub fn current_local(&mut self, local: Local, level: Level) -> Expr {
    assert!(level == Level::level1 || level == Level::level2);
    let symbol =
      if level == Level::level1 {
        self.renaming.current_l1_symbol(self.local_ident(local))
      } else {
        self.renaming.current_l2_symbol(self.local_ident(local), 0)
      };
    let ty = self.function.local_decl(local).0;
    self.ctx.symbol(symbol, ty)
  }

  pub fn new_local(&mut self, local: Local, level: Level) -> Expr {
    assert!(level == Level::level1 || level == Level::level2);
    let symbol =
      if level == Level::level1 {
        self.renaming.new_l1_symbol(self.local_ident(local))
      } else {
        self.renaming.new_l2_symbol(self.local_ident(local), 0)
      };
    let ty = self.function.local_decl(local).0;
    self.ctx.symbol(symbol, ty)
  }

  pub fn new_symbol(&mut self, symbol: &Expr, level: Level) -> Expr {
    assert!(symbol.is_symbol());
    let sym = symbol.symbol();
    let new_sym =
      match level {
        Level::level1 => Some(self.renaming.new_l1_symbol(sym.identifier())),
        Level::level2 => Some(self.renaming.new_l2_symbol(sym.identifier(), 0)),
        _ => None,
      }.expect("Wrong symbol exper");
    self.ctx.symbol(new_sym, symbol.ty())
  }

  pub fn rename(&mut self, expr: &mut Expr, level: Level) {
    match level {
      Level::level0 => return,
      Level::level1 => self.renaming.l1_rename(expr),
      Level::level2 => self.renaming.l2_rename(expr),
    };
  }

  fn constant_propagate(&mut self, lhs: Expr, rhs: Expr) {
    if !rhs.is_constant() && rhs.is_layout() { return; }
    assert!(lhs.is_symbol());
    self.renaming.constant_propagate(lhs, rhs);
  }

  pub fn assignment(&mut self, lhs: Expr, rhs: Expr) {
    // Constant propagation
    self.constant_propagate(lhs.clone(), rhs.clone());

    if rhs.is_layout() { return; }
    
    if lhs.ty().is_any_ptr() {
      let mut l1_lhs = lhs.clone();
      let mut l1_rhs = rhs.clone();
      self.rename(&mut l1_lhs, Level::level1);
      self.rename(&mut l1_rhs, Level::level1);
      let mut objects = HashSet::new();
      self.cur_state.get_value_set(&l1_rhs, &mut objects);
      self.cur_state.update_value_set( l1_lhs, objects, false);
    }
  }
}