
use crate::expr::expr::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_return(&mut self) {
    // TODO: set return value
    
    let n = self.top_mut().function().size();
    let mut state = self.top_mut().cur_state().clone();
    state.remove_stack_places(self.top_mut().function_id());
    self.register_state(n, state);

    self.top_mut().inc_pc();
  }
}