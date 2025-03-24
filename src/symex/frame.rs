use std::collections::*;

use stable_mir::mir::*;

use super::state::*;
use crate::expr::context::*;
use crate::program::function::*;
use crate::symbol::nstring::*;

/// Each frame representing an execution of a function.
/// The id is used for naming variable. It is the unique
/// identifier for each frame.
pub struct Frame<'func> {
    id: usize,
    function: &'func Function,
    _loop_count: HashMap<Pc, usize>,
    /// Previous info. Used for recovering
    pub(super) destination: Option<Place>,
    pub(super) target: Option<BasicBlockIdx>,
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
        target: Option<BasicBlockIdx>,
    ) -> Self {
        let state_map = HashMap::new();
        Frame {
            id,
            function,
            _loop_count: HashMap::new(),
            destination,
            target,
            pc: 0,
            cur_state: State::new(ctx.clone()),
            state_map,
        }
    }

    pub fn cur_pc(&self) -> Option<Pc> {
        if self.pc < self.function.size() { Some(self.pc) } else { None }
    }

    pub fn inc_pc(&mut self) {
        // To handle loop, we set the small pc every time we inc,
        // since the basic blocks have been in reverse post-order.
        self.pc = *self.state_map.keys().min().expect("Impossible");
    }

    // Counting the number of unwinding of loop
    pub fn unwind(&mut self, pc: Pc) {
        if self.function.is_loop_bb(pc) {
            self._loop_count.entry(pc).and_modify(|c| *c += 1).or_insert(1);
            println!(
                "Unwinding loop bb{pc} in {:?} for {} times",
                self.function.name(),
                self._loop_count.get(&pc).unwrap()
            );
        }
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
