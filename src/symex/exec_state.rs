
use std::cell::RefCell;

use stable_mir::mir::*;

use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::program::program::*;
use crate::symbol::symbol::*;
use crate::symbol::nstring::*;
use crate::symex::place_state::*;
use super::frame::*;
use super::namespace::Namespace;
use super::renaming::*;
use super::state::*;
use super::value_set::ObjectSet;

/// Execution state representing the state of the current program.
/// Multi-thread program is not supported yet.
/// 
/// Moreover, `func_cnt` is used for identifying each function. It
/// is used for naming variables later. 
pub struct ExecutionState<'cfg> {
  program: &'cfg Program,
  ctx: ExprCtx,
  pub(super) ns: Namespace,
  pub(super) objects: Vec<Expr>,
  func_cnt: Vec<usize>,
  frames: Vec<Frame<'cfg>>,
  pub(super) renaming: RefCell<Renaming>,
}

impl<'cfg> ExecutionState<'cfg> {
  pub fn new(program: &'cfg Program, ctx: ExprCtx) -> Self {
    ExecutionState {
      program,
      ctx,
      ns: Namespace::default(),
      objects: Vec::new(),
      func_cnt: vec![0; program.size()],
      frames: Vec::new(),
      renaming: RefCell::new(Renaming::default()),
    }
  }

  pub fn setup(&mut self) {
    // create global variable
    let ty = Type::const_array_type(Type::bool_type());
    let alloc_array_symbol = self.l0_symbol(NString::ALLOC_SYM, ty);
    let alloc_array = self.ctx.object(alloc_array_symbol, Ownership::Own);
    self.ns.insert_object(alloc_array);
    // Initialized stack
    self.push_frame(0, None, None);
    let ctx = self.ctx.clone();
    self.top_mut().add_state(0, State::new(ctx));
  }

  pub fn can_exec(&self) -> bool { !self.frames.is_empty() }

  pub fn new_object(&mut self, ty: Type) -> Expr {
    let name =
      NString::from("heap_object_") + self.objects.len().to_string();
    let symbol = Symbol::new(name, 0, 0, Level::Level0);
    let sym_expr = self.ctx.mk_symbol(symbol, ty);
    // Record the ident
    self.ns.insert_symbol(sym_expr.clone());
    // Create an object not being owned by any variable.
    let object = self.ctx.object(sym_expr, Ownership::Not);
    self.objects.push(object.clone());
    object
  }

  pub fn cur_state(&self) -> &State {
    self.top().cur_state()
  }
  
  pub fn cur_state_mut(&mut self) -> &mut State {
    self.top_mut().cur_state_mut()
  }

  pub fn top(&self) -> &Frame<'cfg> {
    self.frames.last().expect("Empty frame stack")
  }

  pub fn top_mut(&mut self) -> &mut Frame<'cfg> {
    self.frames.last_mut().expect("Empty frame stack")
  }

  pub fn push_frame(
    &mut self,
    i: FunctionIdx,
    destination: Option<Place>,
    target: Option<BasicBlockIdx>
  ) {
    self.func_cnt[i] += 1;
    let mut frame = 
      Frame::new(
        self.ctx.clone(),
        self.func_cnt[i],
        self.program.function(i),
        destination,
        target
      );
    if !self.frames.is_empty() {
      frame.cur_state = self.cur_state().clone();
    }
    self.frames.push(frame);
    // init namspace
    for i in 0..self.top().function().locals().len() { self.l0_local(i); }
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

    // clear namspace
    for i in 0..frame.function().locals().len() {
      let ident = frame.local_ident(i);
      self.ns.remove_symbol(ident);
    }

    self.top_mut().inc_pc();
  }

  pub fn l0_symbol(&mut self, ident: NString, ty: Type) -> Expr {
    if self.ns.containts_symbol(ident) {
      self.ns.lookup_symbol(ident)
    } else {
      let symbol =
        Symbol::new(ident,0,0, Level::Level0);
      let symbol_expr = self.ctx.mk_symbol(symbol, ty);
      self.ns.insert_symbol(symbol_expr.clone());
      symbol_expr
    }
  }

  pub fn new_symbol(&mut self, symbol: &Expr, level: Level) -> Expr {
    assert!(symbol.is_symbol() && level != Level::Level0);
    let sym = symbol.extract_symbol();
    let ident = sym.ident();
    let l1_num = sym.l1_num();
    let new_sym =
      match level {
        Level::Level1
          => self.renaming.borrow_mut().new_l1_symbol(ident),
        Level::Level2
          => self.renaming.borrow_mut().new_l2_symbol(ident, l1_num),
        _ => panic!(),
      };
    self.ctx.mk_symbol(new_sym, symbol.ty())
  }

  pub fn l0_local(&mut self, local: Local) -> Expr {
    let ident = self.top().local_ident(local);
    let ty = self.top().function().local_type(local);
    self.l0_symbol(ident, ty)
  }

  pub fn l1_local_count(&self, local: Local) -> usize {
    let ident = self.top().local_ident(local);
    self.renaming.borrow_mut().l1_count(ident)
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
    assert!(level != Level::Level0);
    let ident = self.top().local_ident(local);
    let symbol =
      if level == Level::Level1 {
        self.renaming.borrow_mut().current_l1_symbol(ident)
      } else {
        self.renaming.borrow_mut().current_l2_symbol(ident, 0)
      };
    let ty = self.top().function().local_type(local);
    self.ctx.mk_symbol(symbol, ty)
  }

  pub fn new_local(&mut self, local: Local, level: Level) -> Expr {
    assert!(level != Level::Level0);
    let ident = self.top().local_ident(local);
    let symbol =
      if level == Level::Level1 {
        self.renaming.borrow_mut().new_l1_symbol(ident)
      } else {
        self.renaming.borrow_mut().new_l2_symbol(ident, 0)
      };
    let ty = self.top().function().local_type(local);
    self.ctx.mk_symbol(symbol, ty)
  }

  pub fn rename(&self, expr: &mut Expr, level: Level) {
    match level {
      Level::Level0 => return,
      Level::Level1 => self.renaming.borrow_mut().l1_rename(expr),
      Level::Level2 => self.renaming.borrow_mut().l2_rename(expr),
    };
  }

  fn constant_propagate(&mut self, lhs: Expr, rhs: Expr) {
    if !rhs.is_constant() && !rhs.is_type() { return; }
    assert!(lhs.is_symbol());
    self.renaming.borrow_mut().constant_propagate(lhs, rhs);
  }

  pub fn update_place_state(&mut self, place: Expr, state: PlaceState) {
    if place.is_symbol() {
      let mut l1_place = place;
      self.rename(&mut l1_place, Level::Level1);
      let ident = l1_place.extract_symbol().l1_name();
      let kind = PlaceKind::from(ident);
      let nplace = NPlace::new(kind, ident);
      self.cur_state_mut().update_place_state(nplace, state);
      return;
    }

    if place.is_address_of() {
      let object = place.extract_object();
      self.update_place_state(object, state);
      return;
    }

    if place.is_object() {
      let inner_object = place.extract_inner_expr();
      self.update_place_state(inner_object, state);
      return;
    }

    panic!("Do not support place state: {place:?}");
  }

  pub fn assign(&mut self, lhs: Expr, rhs: Expr) {
    assert!(lhs.is_symbol()); // TODO: do more jobs?

    // Constant propagation
    self.constant_propagate(lhs.clone(), rhs.clone());
    
    // `Layout` is only used for allocation
    if rhs.is_type() { return; }

    // Update place state
    self.update_place_state(lhs.clone(), PlaceState::Initialized);
    
    // Update value Set
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