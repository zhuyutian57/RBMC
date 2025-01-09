
use stable_mir::mir::*;

use crate::symbol::{symbol::*, nstring::*};
use crate::program::program::*;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use super::frame::*;
use super::state::*;

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
}

impl<'exec> ExecutionState<'exec> {
  pub fn new(program: &'exec Program, ctx: ExprCtx) -> Self {
    ExecutionState {
      program,
      ctx,
      objects: Vec::new(),
      func_cnt: vec![0; program.size()],
      frames: Vec::new(),
    }
  }

  pub fn setup(&mut self) {
    self.func_cnt[0] = 1;
    self.frames.push(
      Frame::new(
        self.ctx.clone(),
        self.func_cnt[0],
        self.program.function(0),
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
      self.ctx.symbol(
        Symbol::new(name, 0, 0, Level::level0),
        ty
      );
    let object = self.ctx.object(symbol);
    self.objects.push(object.clone());
    object
  }

  pub fn cur_frame(&mut self) -> &mut Frame<'exec> {
    self.frames.last_mut().expect("Empty frame stack")
  }

  pub fn merge_states(&mut self, pc: Pc) -> bool {
    let state_vec = self.cur_frame().states_from(pc);
    
    let mut new_state = State::new(self.ctx.clone());
    // TODO: do phi function
    if let Some(states) = state_vec {
      for state in states { new_state.merge(&state); }
      self.cur_frame().cur_state = new_state;
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
    let state = self.cur_frame().cur_state().clone();
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
          self.cur_frame().add_state(*t, state);
        }
      }
    }
    self.cur_frame().inc_pc();
  }
}