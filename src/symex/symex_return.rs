
use crate::expr::expr::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn symex_return(&mut self) {
    // TODO: set return value
    
    let n = self.top().function().size();
    let mut state = self.top().cur_state().clone();
    // TODO: remove local in renaming
    for local in 1..self.top().function().locals().len() {
      let l1_count = self.exec_state.l1_local_count(local);
      for l1_num in 1..l1_count + 1 {
        let l1_local = self.exec_state.l1_local(local, l1_num);
        if l1_local.ty().is_any_ptr() {
          state.remove_pointer(l1_local.clone());
        }
      }
    }
    state.remove_stack_places(self.top().function_id());
    self.register_state(n, state);

    self.top().inc_pc();
  }
}