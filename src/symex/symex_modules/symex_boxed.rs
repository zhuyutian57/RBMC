
use stable_mir::mir::*;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::place_state::PlaceState;
use super::super::symex::*;

/// This mod defines symbolic execution of api in std::boxed.
/// In our memory model, `box` is a special pointer that owns
/// the object it points to. Whenever it is dropped, the object
/// is also dealloced, too. Thus, if we drop an uninitialized
/// box, `Invalid-Free` occurs.

impl<'cfg> Symex<'cfg> {
  pub fn symex_boxed_api(
    &mut self,
    fndef: &FunctionDef,
    args: &Vec<Operand>,
    dest: &Place,
  ) {
    let name = NString::from(fndef.0.trimmed_name());
    if name == NString::from("Box::<T>::new") {
      self.symex_box_new(dest, args);
    } else {
      panic!("Not support {name:?}");
    }
  }

  fn symex_box_new(&mut self, dest: &Place, args: &Vec<Operand>) {
    let lhs = self.make_project(dest);
    let ty = lhs.ty().pointee_ty();
    let object = self.exec_state.new_object(ty);

    // Assign value
    let value = self.make_operand(&args[0]);
    self.assign(object.clone(), value, self.ctx._true().into());
    
    // Return box pointer
    let address_of =
      self.ctx.address_of(object.clone(), object.extract_address_type());
    let _box = self.ctx._box(address_of);
    self.assign(lhs, _box, self.ctx._true().into());

    // Track new object
    self.track_new_object(object.clone());

    // The newly object is owned by the box pointer
    let place_state = PlaceState::Own;
    self.exec_state.update_place_state(object, place_state);
  }
}