
use std::fmt::Error;

use stable_mir::mir::*;
use stable_mir::ty::*;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::*;
use crate::NString;
use super::place_state::*;
use super::symex::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AllocKind {
  Alloc,
  Box,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FnKind {
  Unwind(FunctionIdx),
  Layout(Type),
  Allocation(AllocKind),
  Dealloc,
  PtrEq,
}

impl <'cfg> Symex<'cfg> {
  pub(super) fn symex_call(
    &mut self,
    func: &Operand,
    args: &Vec<Operand>,
    dest: &Place,
    target: &Option<BasicBlockIdx>
  ) {
    let ty = self.top_mut().function().operand_type(func);
    let fndef = ty.fn_def();
    let fnkind = self.make_fn_kind(fndef);
    match &fnkind {
        FnKind::Unwind(i)
          => self.symex_function(*i, args, dest, target),
        FnKind::Layout(t)
          => self.symex_assign_layout(dest, *t),
        FnKind::Allocation(k)
          => self.allocation(dest, args, *k),
        FnKind::Dealloc => {
          let mut pt = self.make_operand(&args[0]);
          self.replace_predicates(&mut pt);
          self.symex_dealloc(pt);
        },
        FnKind::PtrEq => self.ptr_eq(dest, args),
    };
    if matches!(fnkind, FnKind::Unwind(_)) { return; }
    if let Some(t) = target {
      let state = self.top_mut().cur_state().clone();
      self.register_state(*t, state);
      self.top_mut().inc_pc();
    }
  }

  fn make_fn_kind(&mut self, fndef: (FnDef, GenericArgs)) -> FnKind {
    let name = NString::from(fndef.0.name());
    let trimmed_name = NString::from(fndef.0.trimmed_name());
    if self.program.contains_function(trimmed_name) {
      Ok(FnKind::Unwind(self.program.function_idx(trimmed_name)))
    } else if trimmed_name == NString::from("Layout::new") {
      assert!(fndef.1.0.len() == 1);
      let ty = fndef.1.0[0].ty().unwrap();
      Ok(FnKind::Layout(Type::from(*ty)))
    } else if trimmed_name == NString::from("Box::<T>::new") {
      Ok(FnKind::Allocation(AllocKind::Box))
    } else if trimmed_name == NString::from("alloc") {
      Ok(FnKind::Allocation(AllocKind::Alloc))
    } else if trimmed_name == NString::from("dealloc") {
      Ok(FnKind::Dealloc)
    } else if trimmed_name == NString::from("eq") {
      assert!(name.contains(NString::from("std::ptr")));
      Ok(FnKind::PtrEq)
    } else {
      Err(Error)
    }.expect(format!("Do not support {name:?}").as_str())
  }

  fn symex_function(
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
    let args = self.top_mut().function().args();
    if !args.is_empty() {
      for arg_local in args.iter() {
        let lhs = self.exec_state.l0_local(*arg_local);
        let rhs = arg_exprs[*arg_local - 1].clone();
        self.assign(lhs, rhs, self.ctx._true());
      }
    }
    let state = self.top_mut().cur_state().clone();
    self.register_state(0, state);
  }

  fn allocation(
    &mut self,
    dest: &Place,
    args: &Vec<Operand>,
    k: AllocKind
  ) {
    let ty = self.make_type(&args[0]);
    let object = self.symex_alloc(ty);
    let pt = self.make_project(dest);
    let address_of = self.ctx.address_of(object.clone(), pt.ty());
    
    self.assign(pt, address_of, self.ctx._true());

    let place_state =
      if matches!(k, AllocKind::Box) {
        let value = self.make_operand(&args[0]);
        self.assign(object.clone(), value, self.ctx._true());
        PlaceState::Own
      } else {
        PlaceState::Alloced
      };
    self.exec_state.update_place_state(object, place_state);
  }

  fn ptr_eq(&mut self, dest: &Place, args: &Vec<Operand>) {
    assert!(args.len() == 2);
    let lhs = self.make_project(dest);
    
    let p1 = self.make_operand(&args[0]);
    let p2 = self.make_operand(&args[1]);
    let mut rhs = self.ctx.eq(p1, p2);
    self.replace_predicates(&mut rhs);

    self.assign(lhs, rhs, self.ctx._true());
  }

  pub(super) fn symex_return(&mut self) {
    let n = self.top_mut().function().size();
    let state = self.top_mut().cur_state().clone();
    self.register_state(n, state);

    self.top_mut().inc_pc();
  }

  pub(super) fn symex_end_function(&mut self) {
    let pc = self.top().function().size();
    // Must exist
    assert!(self.merge_states(pc));

    let frame = self.exec_state.pop_frame();
    if !self.exec_state.can_exec() { return; }

    self.top_mut().cur_state = frame.cur_state.clone();
    
    // Assign return value
    if let Some(ret) = &frame.destination {
      let lhs = self.make_project(ret);
      let rhs_ident = frame.local_ident(0);
      let rhs_ty = frame.function().local_type(0);
      let rhs = self.exec_state.l0_symbol(rhs_ident, rhs_ty);
      self.assign(lhs, rhs, self.ctx._true());
    }

    if let Some(t) = &frame.target {
      let mut state = self.top().cur_state.clone();
      state.remove_stack_places(frame.function_id());
      self.top_mut().add_state(*t, state);
    }

    // clear namspace
    self.exec_state.ns.clear_local_symbols(frame.function_id());

    // clear renaming
    self.exec_state.renaming.borrow_mut().cleanr_locals(frame.function_id());

    self.top_mut().inc_pc();
  }
}