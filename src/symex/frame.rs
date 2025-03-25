use std::collections::*;

use stable_mir::mir::*;

use super::state::*;
use crate::config::config::Config;
use crate::expr::context::*;
use crate::program::function::*;
use crate::symbol::nstring::*;

/// Each frame representing an execution of a function.
/// The id is used for naming variable. It is the unique
/// identifier for each frame.
pub struct Frame<'func> {
    config: &'func Config,
    id: usize,
    function: &'func Function,
    _loop_count: HashMap<Pc, usize>,
    /// Previous info. Used for recovering
    pub(super) destination: Option<Place>,
    pub(super) target: Option<BasicBlockIdx>,
    /// Current Computing
    pc: Pc,
    loop_stack: Vec<Pc>,
    pub(super) cur_state: State,
    state_map: HashMap<Pc, Vec<State>>,
}

impl<'func> Frame<'func> {
    pub fn new(
        config: &'func Config,
        id: usize,
        function: &'func Function,
        destination: Option<Place>,
        target: Option<BasicBlockIdx>,
    ) -> Self {
        Frame {
            config,
            id,
            function,
            _loop_count: HashMap::new(),
            destination,
            target,
            pc: 0,
            loop_stack: Vec::new(),
            cur_state: State::new(config.expr_ctx.clone()),
            state_map: HashMap::new(),
        }
    }

    pub fn cur_pc(&self) -> Option<Pc> {
        if self.pc < self.function.size() { Some(self.pc) } else { None }
    }

    pub fn inc_pc(&mut self) {
        while let Some(&l) = self.loop_stack.last() {
            let mut mi = self.function.size();
            for pc in self.function.get_loop(l) {
                if self.state_map.contains_key(&pc) {
                    mi = std::cmp::min(mi, *pc);
                }
            }
            if mi != self.function.size() {
                self.pc = mi;
                return;
            } else {
                self.loop_stack.pop();
            }
        }
        
        self.pc = *self.state_map.keys().min().unwrap();
    }

    /// Counting the number of unwinding of loop
    pub fn unwind(&mut self, pc: Pc) {
        if self.function.is_loop_bb(pc) {
            self._loop_count.entry(pc).and_modify(|c| *c += 1).or_insert(1);
            self.loop_stack.push(pc);
            println!(
                "Unwinding loop bb{pc} in {:?} for {} times",
                self.function.name(),
                self._loop_count.get(&pc).unwrap()
            );
        }
    }

    /// Check whether the current loop read loop bound
    pub fn reach_loop_bound(&self, pc: Pc) -> bool {
        self.config.cli.unwind != 0 && self._loop_count.contains_key(&pc) &&
            *self._loop_count.get(&pc).unwrap() >= self.config.cli.unwind
    }

    pub fn cur_state(&self) -> &State {
        &self.cur_state
    }

    pub fn cur_state_mut(&mut self) -> &mut State {
        &mut self.cur_state
    }

    pub fn add_state(&mut self, pc: Pc, state: State) {
        self.state_map.entry(pc).or_default().push(state);
    }

    pub fn states_from(&mut self, pc: Pc) -> Option<Vec<State>> {
        self.state_map.remove(&pc)
    }

    pub fn function(&self) -> &'func Function {
        self.function
    }

    pub fn function_id(&self) -> NString {
        self.function.name() + "_" + self.id.to_string()
    }

    pub fn local_ident(&self, local: Local) -> NString {
        self.function_id() + "::" + local.to_string()
    }
}
