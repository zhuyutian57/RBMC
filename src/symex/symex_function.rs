
use std::fmt::Error;

use stable_mir::mir::*;
use stable_mir::ty::*;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::*;
use crate::symbol::symbol::Level;
use crate::NString;
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
    let ty = self.top_mut().function().operand_type(func);
    let fndef = ty.fn_def();
    let fnkind = self.make_fn_kind(fndef, args);
    match &fnkind {
        FnKind::Unwind(i) => self.symex_function(*i, args, dest, target),
        FnKind::Layout(l) => self.symex_assign_layout(dest, *l),
        FnKind::Allocation(k, t) => {
          let object = self.symex_alloc(*t, *k);
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
        },
        _ => panic!("Need implement"),
    };
    if matches!(fnkind, FnKind::Unwind(_)) { return; }
    if let Some(t) = target {
      let state = self.top_mut().cur_state().clone();
      self.register_state(*t, state);
      self.top_mut().inc_pc();
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

  fn make_fn_kind(
    &mut self,
    fndef: (FnDef, GenericArgs),
    args: &Vec<Operand>
  ) -> FnKind {
    let name = NString::from(fndef.0.trimmed_name());
    if self.program.contains_function(name) {
      Ok(FnKind::Unwind(self.program.function_idx(name)))
    } else if name == NString::from("Layout::new") {
      assert!(fndef.1.0.len() == 1);
      let ty = fndef.1.0[0].ty().unwrap();
      Ok(FnKind::Layout(Type::from(*ty)))
    } else if name == NString::from("Box::<T>::new") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
      Ok(FnKind::Allocation(AllocKind::Box, ty))
    } else if name == NString::from("alloc") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
      Ok(FnKind::Allocation(AllocKind::Alloc, ty))
    } else if name == NString::from("AsMut::as_mut") {
      Ok(FnKind::AsMut(args[0].clone()))
    } else {
      Err(Error)
    }.expect(format!("Do not support {name:?}").as_str())
  }
}