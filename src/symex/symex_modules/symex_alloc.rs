
use stable_mir::mir::*;
use stable_mir::ty::FnDef;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::place_state::PlaceState;
use crate::symex::projection::Mode;
use crate::symex::value_set::ObjectSet;
use super::super::symex::*;

/// This mod defines symbolic execution of api in std::alloc

impl<'cfg> Symex<'cfg> {
  pub fn symex_alloc_api(
    &mut self,
    fndef: &FunctionDef,
    args: &Vec<Operand>,
    dest: &Place,
  ) {
    let name = NString::from(fndef.0.trimmed_name());
    if name == NString::from("alloc") {
      self.symex_alloc(dest, args);
    } else if name == NString::from("dealloc") {
      self.symex_dealloc(&args[0]);
    } else if name == NString::from("Layout::new") {
      self.symex_layout_new(dest, fndef);
    } else {
      panic!("Not support {name:?}");
    }
  }

  fn symex_alloc(&mut self, dest: &Place, args: &Vec<Operand>) {
    let mut layout = self.make_operand(&args[0]);
    self.replace_predicates(&mut layout);
    self.rename(&mut layout);
    assert!(layout.is_type());
    let object =  self.exec_state.new_object(layout.extract_type());

    let lhs = self.make_project(dest);
    let address_of = self.ctx.address_of(object.clone(), lhs.ty());
    
    self.assign(lhs, address_of, self.ctx._true().into());

    self.track_new_object(object.clone());

    let place_state = PlaceState::Alloced;
    self.exec_state.update_place_state(object, place_state);
  }

  fn symex_dealloc(&mut self, operand: &Operand) {
    let pt = self.make_operand(operand);
    assert!(pt.ty().is_ptr());
    // Generate assertions
    self.make_deref(pt.clone(), Mode::Dealloc, self.ctx._true().into());

    self.top_mut().cur_state.dealloc_objects(pt.clone());
    self.top_mut().cur_state.remove_pointer(pt.clone());

    let pointer_base = self.ctx.pointer_base(pt);
    let alloc_array =
      self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
    let index =
      self.ctx.index(alloc_array, pointer_base, Type::bool_type());
    self.assign(index, self.ctx._false(), self.ctx._true().into());
  }

  fn symex_layout_new(&mut self, dest: &Place, fndef: &FunctionDef) {
    let ty = Type::from(fndef.1.0[0].expect_ty());
    self.symex_assign_layout(dest, ty);
  }
}