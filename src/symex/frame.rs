
use std::collections::*;

use stable_mir::mir::*;

use crate::symbol::nstring::*;
use crate::program::program::*;
use crate::expr::context::*;
use super::state::*;

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
}

impl<'func> Frame<'func> {
  pub fn new(
    ctx: ExprCtx,
    id: usize,
    function: &'func Function,
    destination: Option<Place>,
    target: Option<BasicBlockIdx>
  ) -> Self {
    let mut state_map = HashMap::new();
    Frame {
      ctx: ctx.clone(),
      id, function,
      destination,
      target,
      pc: 0,
      cur_state: State::new(ctx.clone()),
      state_map,
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
    // println!(
    //   "Done {:?} - bb{}\n{:?}",
    //   self.function.name(),
    //   self.pc,
    //   self.cur_state
    // );
    self.pc += 1;
  }

  pub fn cur_state(&self) -> &State { &self.cur_state }

  pub fn cur_state_mut(&mut self) -> &mut State { &mut self.cur_state }

  pub fn add_state(&mut self, pc: Pc, state: State) {
    self.state_map.entry(pc).or_default().push(state);
  }

  pub fn states_from(&mut self, pc: Pc) -> Option<Vec<State>> {
    self.state_map.remove(&pc)
  }

  pub fn function(&self) -> &'func Function {
    self.function
  }

  pub fn local_ident(&self, local: Local) -> NString {
    self.function.name()
      + "_" + self.id.to_string()
      + "::" + local.to_string()
  }
}