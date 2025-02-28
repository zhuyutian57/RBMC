
use stable_mir::mir::*;

use crate::expr::expr::*;
use crate::program::program::*;
use super::place_state::*;
use super::symex::*;

impl <'cfg> Symex<'cfg> {
  pub(super) fn symex_call(
    &mut self,
    func: &Operand,
    args: &Vec<Operand>,
    dest: &Place,
    target: &Option<BasicBlockIdx>
  ) {
    let ty = self.top().function().operand_type(func);
    let fndef = ty.fn_def();
    let fnkind = self.make_fn_kind(fndef, args);
    match &fnkind {
        FnKind::Unwind(i) => self.symex_function(*i, args, dest, target),
        FnKind::Layout(l) => self.symex_assign_layout(dest, *l),
        FnKind::Allocation(k, t) => {
          let object = self.symex_alloc(*t, *k);
          let pt = self.make_project(dest);
          let address_of = self.ctx.address_of(object.clone(), pt.ty());
          
          self.assign(pt, address_of, self.ctx.constant_bool(true));
          
          // TODO - do assignment for constant

          let object_state =
            if matches!(k, AllocKind::Box) {
              PlaceState::Initialized
            } else {
              PlaceState::Uninitialized
            };
          self.exec_state.update_place_state(object, object_state);
        },
        _ => panic!("Need implement"),
    };
    if matches!(fnkind, FnKind::Unwind(_)) { return; }
    if let Some(t) = target {
      let state = self.top().cur_state().clone();
      self.register_state(*t, state);
      self.top().inc_pc();
    }
  }

  pub(super) fn symex_function(
    &mut self,
    i: FunctionIdx,
    args: &Vec<Operand>,
    dest: &Place,
    target: &Option<BasicBlockIdx>
  ) {
    let mut arg_exprs = Vec::new();
    for arg in args {
      arg_exprs.push(self.make_operand(arg));
    }
    // push frame for new name
    self
      .exec_state
      .push_frame(i, Some(dest.clone()), *target);
    // Set arguements
    let args = self.top().function().args();
    if !args.is_empty() {
      for arg_local in args.iter() {
        let lhs = self.exec_state.l0_local(*arg_local);
        let rhs = arg_exprs[*arg_local - 1].clone();
        self.assign(lhs, rhs, self.ctx.constant_bool(true));
      }
    }
    let state = self.top().cur_state().clone();
    self.register_state(0, state);
  }
}