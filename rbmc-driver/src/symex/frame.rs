use std::collections::*;

use stable_mir::mir::*;

use super::place_state::PlaceState;
use super::state::*;
use crate::program::function::*;
use crate::symbol::nstring::*;
use crate::symbol::symbol;
use crate::symbol::symbol::*;

/// Each frame representing an execution of a function.
/// The id is used for naming variable. It is the unique
/// identifier for each frame.
pub struct Frame<'func> {
    pub(super) id: usize,
    pub(super) function: &'func Function,
    /// Previous info. Used for recovering
    pub(super) dest: Option<Place>,
    pub(super) target: Option<BasicBlockIdx>,
    /// Current Computing
    pub(super) pc: Pc,
    /// Record l1 number of each local and its liveness
    pub(super) local_states: Vec<(usize, bool)>,
    pub(super) loop_stack: Vec<(Pc, usize)>,
    pub(super) unexplored_states: HashMap<Pc, Vec<State>>,
}

impl<'func> Frame<'func> {
    pub fn new(
        id: usize,
        function: &'func Function,
        dest: Option<Place>,
        target: Option<BasicBlockIdx>,
    ) -> Self {
        Frame {
            id,
            function,
            dest,
            target,
            pc: 0,
            local_states: vec![(0, false); function.locals().len()],
            loop_stack: vec![],
            unexplored_states: HashMap::new(),
        }
    }

    pub fn get_local_place_state(&self, symbol: Symbol) -> PlaceState {
        assert!(symbol.is_stack_symbol());
        let local = symbol.local();
        let l1_num = symbol.l1_num();
        let &(c, s) = &self.local_states[local];
        if  c== l1_num && s { PlaceState::Own } else { PlaceState::Dead }
    }

    pub fn add_state(&mut self, pc: Pc, state: State) {
        self.unexplored_states.entry(pc).or_default().push(state);
    }

    pub fn unexplored_states_from(&mut self, pc: Pc) -> Option<Vec<State>> {
        self.unexplored_states.remove(&pc)
    }

    pub fn frame_ident(&self) -> NString {
        self.function.name() + "_" + self.id.to_string()
    }

    pub fn local_ident(&self, local: Local) -> Ident {
        Ident::Stack(self.function.name(), self.id, local)
    }
}
