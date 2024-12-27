
use std::fmt::Debug;
use std::{clone, vec};
use std::{collections::*, hash::Hash, rc::Rc};
use std::cell::{RefCell, RefMut};

use stable_mir::mir::*;
use stable_mir::ty::*;

use crate::expr::symbol::Symbol;
use crate::nstring::NString;
use crate::program::*;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::renaming::Renaming;

pub type PointsToSet = HashMap<Expr, HashSet<Expr>>;

/// Abstract program state for each program point
#[derive(Clone)]
pub struct State {
  guard: Expr,
  points_to: PointsToSet,
}

impl State {
  pub fn new(ctx: ExprCtx) -> Self {
    State {
      guard: ctx.constant_bool(true),
      points_to: HashMap::new(),
    }
  }

  pub fn guard(&self) -> Expr { self.guard.clone() }

  pub fn points_to_object(&mut self, pt: Expr, object: Expr) {
    self.points_to.entry(pt).or_default().insert(object);
  }

  pub fn points_to(&self) -> &PointsToSet { &self.points_to }

  pub fn contains_pointer(&self, pt: &Expr) -> bool {
    self.points_to.contains_key(pt)
  }

  pub fn add_pointer(&mut self, pt: Expr) {
    self.points_to.entry(pt).or_default();
  }

  pub fn remove_pointer(&mut self, pt: Expr) {
    self.points_to.remove(&pt);
  }
}

impl Debug for State {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut pts = String::new();
    for (s, objects) in self.points_to.iter() {
      pts.push_str(format!("  {s:?}: {objects:?}\n").as_str());
    }
    write!(f, "State -> Guard: {:?}\n{}", self.guard, pts)
  }
}

pub type Pc = BasicBlockIdx;

/// Each frame representing an execution of a function.
/// The id is used for naming variable. It is the unique
/// identifier for each frame.
pub struct Frame<'func> {
  ctx: ExprCtx,

  id: usize,
  function: &'func Function,

  destination: Option<Place>,
  target: Option<BasicBlockIdx>,
  
  pc: Pc,
  cur_state: State,
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
  ) -> Self {
    let mut state_map = HashMap::new();
    state_map.insert(0, vec![State::new(ctx.clone())]);
    Frame {
      ctx: ctx.clone(),
      id, function,
      destination,
      target,
      pc: 0,
      cur_state: State::new(ctx),
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

  pub fn inc_pc(&mut self) { self.pc += 1; }

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

  pub fn operand_ty(&self, operand: &Operand) -> Ty {
    operand
    .ty(self.function.body().locals())
    .expect("Wrong operand")
  }

  fn local_identifier(&self, local: Local) -> NString {
    self.function.name()
      + "_" + self.id.to_string()
      + "::" + local.to_string()
  }

  pub fn current_local(&mut self, local: Local, level2: bool) -> Expr {
    let symbol =
      if !level2 {
        self.renaming.l1_symbol(self.local_identifier(local))
      } else {
        self.renaming.l2_symbol(self.local_identifier(local))
      };
    let ty = self.function.local_decl(local).0;
    self.ctx.symbol(symbol, ty)
  }

  pub fn new_local(&mut self, local: Local, level2: bool) -> Expr {
    let symbol =
      if !level2 {
        self.renaming.new_l1_symbol(self.local_identifier(local))
      } else {
        self.renaming.new_l2_symbol(self.local_identifier(local))
      };
    let ty = self.function.local_decl(local).0;
    self.ctx.symbol(symbol, ty)
  }

  pub fn rename(&mut self, expr: &mut Expr, level2: bool) {
    self.renaming.l1_rename(expr);
    if level2 {
      self.renaming.l2_rename(expr);
    }
  }

  pub fn constant_propagate(&mut self, symbol: Symbol, constant: Expr) {
    self.renaming.constant_propagate(symbol, constant);
  }
}

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
        None
      )
    );
  }

  pub fn can_exec(&self) -> bool { !self.frames.is_empty() }

  pub fn new_object(&mut self, ty: Type) -> Expr {
    let name =
      NString::from("heap_object_") + self.objects.len().to_string();
    let symbol = 
      self.ctx.symbol(
        Symbol::new(name, 0, 0, 0),
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
    let mut state_vec = self.cur_frame().states_from(pc);
    
    let mut new_state = State::new(self.ctx.clone());
    // TODO: do phi function
    if let Some(states) = state_vec {
      for state in states {
        new_state.guard =
          self.ctx.or(new_state.guard, state.guard);
        for (pt, objects) in state.points_to.iter() {
          new_state.add_pointer(pt.clone());
          for object in objects.iter() {
            new_state.points_to_object(pt.clone(), object.clone());
          }
        }
      }
      new_state.guard.simplify();
      self.cur_frame().cur_state = new_state;
      true
    } else {
      false
    }
  }

  pub fn push_frame(
    &mut self,
    i: FunctionIdx,
    args: &Vec<Operand>,
    destination: Place,
    target: Option<BasicBlockIdx>
  ) {
    // TODO - do something with arguments
    self.frames.push(
      Frame::new(
        self.ctx.clone(),
        self.func_cnt[i],
        self.program.function(i),
        Some(destination),
        target
      ));
  }

  pub fn pop_frame(&mut self) {
    assert!(!self.frames.is_empty());
    let frame = self.frames.pop().unwrap();
    if self.frames.is_empty() { return; }
    
    // TODO

    if let Some(t) = frame.target() {
      let state = self.cur_frame().cur_state().clone();
      self.cur_frame().add_state(*t, state);
    }
    self.cur_frame().inc_pc();
  }
}