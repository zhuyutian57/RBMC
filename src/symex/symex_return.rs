
use crate::expr::expr::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_return(&mut self) {
    // TODO: set return value
    
    let n = self.top().function().size();
    let mut state = self.top().cur_state().clone();
    state.remove_stack_places(self.top().function_id());
    self.register_state(n, state);

    self.top().inc_pc();
  }
}