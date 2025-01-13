
use stable_mir::mir::*;

use crate::symbol::{symbol::*, nstring::*};
use crate::program::program::*;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use super::frame::*;
use super::renaming::*;
use super::state::*;
use super::value_set::ObjectSet;

/// Execution state representing the state of the current program.
/// Multi-thread program is not supported yet.
/// 
/// Moreover, `func_cnt` is used for identifying each function. It
/// is used for naming variables later. 
pub struct ExecutionState<'exec> {
  program: &'exec Program,
  ctx: ExprCtx,
  objects: Vec<Expr>,
  func_cnt: Vec<usize>,
  frames: Vec<Frame<'exec>>,
  renaming: Renaming,
}

impl<'exec> ExecutionState<'exec> {
  pub fn new(program: &'exec Program, ctx: ExprCtx) -> Self {
    ExecutionState {
      program,
      ctx,
      objects: Vec::new(),
      func_cnt: vec![0; program.size()],
      frames: Vec::new(),
      renaming: Renaming::default(),
    }
  }

  pub fn setup(&mut self) {
    self.func_cnt[0] = 1;
    self.frames.push(
      Frame::new(
        self.ctx.clone(),
        self.func_cnt[0],
        self.program.entry_fn(),
        None,
        None,
        State::new(self.ctx.clone())
      )
    );
  }

  pub fn can_exec(&self) -> bool { !self.frames.is_empty() }

  pub fn new_object(&mut self, ty: Type) -> Expr {
    let name =
      NString::from("heap_object_") + self.objects.len().to_string();
    let symbol = 
      self.ctx.mk_symbol(
        Symbol::new(name, 0, 0, Level::Level0),
        ty
      );
    let object = self.ctx.object(symbol);
    self.objects.push(object.clone());
    object
  }

  pub fn cur_state(&self) -> &State {
    self.top().cur_state()
  }
  
  pub fn cur_state_mut(&mut self) -> &mut State {
    self.top_mut().cur_state_mut()
  }

  pub fn top(&self) -> &Frame<'exec> {
    self.frames.last().expect("Empty frame stack")
  }

  pub fn top_mut(&mut self) -> &mut Frame<'exec> {
    self.frames.last_mut().expect("Empty frame stack")
  }

  pub fn merge_states(&mut self, pc: Pc) -> bool {
    let state_vec = self.top_mut().states_from(pc);
    
    let mut new_state = State::new(self.ctx.clone());
    // TODO: do phi function
    if let Some(states) = state_vec {
      for state in states { new_state.merge(&state); }
      self.top_mut().cur_state = new_state;
      true
    } else {
      false
    }
  }

  pub fn push_frame(
    &mut self,
    i: FunctionIdx,
    destination: Place,
    target: Option<BasicBlockIdx>
  ) {
    let state = self.top_mut().cur_state().clone();
    self.frames.push(
      Frame::new(
        self.ctx.clone(),
        self.func_cnt[i],
        self.program.function(i),
        Some(destination),
        target,
        state
      ));
  }

  pub fn pop_frame(&mut self) {
    assert!(!self.frames.is_empty());
    let mut frame = self.frames.pop().unwrap();
    if self.frames.is_empty() { return; }

    let new_states =
      frame.states_from(frame.function().size());

    if let Some(t) = frame.target() {
      if let Some(states) = new_states {
        for state in states {
          self.top_mut().add_state(*t, state);
        }
      }
    }
    self.top_mut().inc_pc();
  }

  pub fn l0_local(&self, local: Local) -> Expr {
    let ident = self.top().local_ident(local);
    let symbol = Symbol::new(ident,0,0, Level::Level0);
    let ty = self.top().function().local_type(local);
    self.ctx.mk_symbol(symbol, ty)
  }

  pub fn l1_local_count(&self, local: Local) -> usize {
    let ident = self.top().local_ident(local);
    self.renaming.count(ident, Level::Level1)
  }

  pub fn l1_local(&self, local: Local, mut l1_num: usize) -> Expr {
    if l1_num == 0 { l1_num = self.l1_local_count(local); }
    assert!(0 < l1_num && l1_num <= self.l1_local_count(local));
    let ident = self.top().local_ident(local);
    let symbol =
      Symbol::new(ident, l1_num, 0, Level::Level1);
    let ty = self.top().function().local_type(local);
    self.ctx.mk_symbol(symbol, ty)
  }

  pub fn current_local(&mut self, local: Local, level: Level) -> Expr {
    assert!(level == Level::Level1 || level == Level::Level2);
    let ident = self.top().local_ident(local);
    let symbol =
      if level == Level::Level1 {
        self.renaming.current_l1_symbol(ident)
      } else {
        self.renaming.current_l2_symbol(ident, 0)
      };
    let ty = self.top().function().local_type(local);
    self.ctx.mk_symbol(symbol, ty)
  }

  pub fn new_local(&mut self, local: Local, level: Level) -> Expr {
    assert!(level == Level::Level1 || level == Level::Level2);
    let ident = self.top().local_ident(local);
    let symbol =
      if level == Level::Level1 {
        self.renaming.new_l1_symbol(ident)
      } else {
        self.renaming.new_l2_symbol(ident, 0)
      };
    let ty = self.top().function().local_type(local);
    self.ctx.mk_symbol(symbol, ty)
  }

  pub fn new_symbol(&mut self, symbol: &Expr, level: Level) -> Expr {
    assert!(symbol.is_symbol());
    let sym = symbol.extract_symbol();
    let ident = sym.identifier();
    let new_sym =
      match level {
        Level::Level1 => Some(self.renaming.new_l1_symbol(ident)),
        Level::Level2 => Some(self.renaming.new_l2_symbol(ident, 0)),
        _ => None,
      }.expect("Wrong symbol exper");
    self.ctx.mk_symbol(new_sym, symbol.ty())
  }

  pub fn rename(&mut self, expr: &mut Expr, level: Level) {
    match level {
      Level::Level0 => return,
      Level::Level1 => self.renaming.l1_rename(expr),
      Level::Level2 => self.renaming.l2_rename(expr),
    };
  }

  fn constant_propagate(&mut self, lhs: Expr, rhs: Expr) {
    if !rhs.is_constant() && !rhs.is_type() { return; }
    assert!(lhs.is_symbol());
    self.renaming.constant_propagate(lhs, rhs);
  }

  pub fn assign(&mut self, lhs: Expr, rhs: Expr) {
    // Constant propagation
    self.constant_propagate(lhs.clone(), rhs.clone());

    if rhs.is_type() { return; }
    
    if lhs.ty().is_any_ptr() {
      let mut l1_lhs = lhs.clone();
      let mut l1_rhs = rhs.clone();
      self.rename(&mut l1_lhs, Level::Level1);
      self.rename(&mut l1_rhs, Level::Level1);
      let mut objects = ObjectSet::new();
      self.cur_state().get_value_set(l1_rhs.clone(), &mut objects);
      self.cur_state_mut().assign(l1_lhs, objects);
    }
  }
}