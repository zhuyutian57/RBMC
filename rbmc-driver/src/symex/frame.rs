use std::collections::*;

use stable_mir::mir::*;

use super::state::*;
use crate::config::config::Config;
use crate::program::function::*;
use crate::symbol::nstring::*;

/// Each frame representing an execution of a function.
/// The id is used for naming variable. It is the unique
/// identifier for each frame.
pub struct Frame<'func> {
    id: usize,
    config: &'func Config,
    pub(super) function: &'func Function,
    /// Previous info. Used for recovering
    pub(super) dest: Option<Place>,
    pub(super) target: Option<BasicBlockIdx>,
    /// Current Computing
    pc: Pc,
    pub(super) loop_stack: Vec<(Pc, usize)>,
    pub(super) cur_state: State,
    state_map: HashMap<Pc, Vec<State>>,
}

impl<'func> Frame<'func> {
    pub fn new(
        id: usize,
        config: &'func Config,
        function: &'func Function,
        dest: Option<Place>,
        target: Option<BasicBlockIdx>,
    ) -> Self {
        Frame {
            id,
            config,
            function,
            dest,
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
        if let Some(&(l, _)) = self.loop_stack.last() {
            let mut mi = self.function.size();
            for pc in self.function.get_loop(l) {
                if self.state_map.contains_key(pc) {
                    mi = std::cmp::min(mi, *pc);
                }
            }
            if mi != self.function.size() {
                self.pc = mi;
                return;
            }
        }

        if self.state_map.is_empty() {
            panic!("We stuck in a loop, please increase the loop bound");
        }

        self.pc = *self.state_map.keys().min().unwrap();
    }

    pub fn new_loop(&mut self, pc: Pc) {
        assert!(self.function.is_loop_bb(pc));
        self.loop_stack.push((pc, 1));
    }

    pub fn cur_loop(&self) -> Option<&(Pc, usize)> {
        self.loop_stack.last()
    }

    pub fn cur_loop_mut(&mut self) -> Option<&mut (Pc, usize)> {
        self.loop_stack.last_mut()
    }

    /// Check whether the current loop read loop bound
    pub fn reach_loop_bound(&self) -> bool {
        self.config.cli.unwind != 0
            && !self.loop_stack.is_empty()
            && self.loop_stack.last().unwrap().1 > self.config.cli.unwind
    }

    pub fn add_state(&mut self, pc: Pc, state: State) {
        self.state_map.entry(pc).or_default().push(state);
    }

    pub fn states_from(&mut self, pc: Pc) -> Option<Vec<State>> {
        self.state_map.remove(&pc)
    }

    pub fn function_id(&self) -> NString {
        self.function.name() + "_" + self.id.to_string()
    }

    pub fn local_ident(&self, local: Local) -> NString {
        self.function_id() + "::" + local.to_string()
    }
}
