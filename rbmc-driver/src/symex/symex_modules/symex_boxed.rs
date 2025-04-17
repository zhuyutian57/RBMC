use stable_mir::mir::mono::Instance;
use stable_mir::CrateDef;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::place_state::NPlace;
use crate::symex::place_state::PlaceState;
use crate::symex::value_set::ObjectSet;

/// This mod defines symbolic execution of api in std::boxed.
/// In our memory model, `box` is a special pointer that owns
/// the object it points to. Whenever it is dropped, the object
/// is also dealloced, too. Thus, if we drop an uninitialized
/// box, `Invalid-Free` occurs.

impl<'cfg> Symex<'cfg> {
    pub fn symex_boxed_api(&mut self, instance: Instance, args: Vec<Expr>, dest: Expr) {
        let fty = Type::from(instance.ty());
        let name = NString::from(fty.fn_def().0.trimmed_name());
        if name == "Box::<T>::new" {
            self.symex_box_new(dest, args);
        // } else if name == "Box::<T>::from_raw" {
        //     self.symex_box_from_raw(dest, args);
        // } else if name == "Box::<T, A>::into_raw" {
        //     self.symex_box_into_raw(dest, args);
        } else {
            panic!("Not support {name:?}");
        }
    }

    fn symex_box_new(&mut self, dest: Expr, args: Vec<Expr>) {
        let lhs = dest.clone();
        let ty = lhs.ty().pointee_ty();
        let object = self.exec_state.new_object(ty);

        // Assign value
        let value = args[0].clone();
        self.assign(object.clone(), value, self.ctx._true().into());

        // Construct box pointer
        let address = self.ctx.address_of(object.clone(), object.extract_address_type());
        let _box = self.ctx._box(address);
        self.assign(lhs, _box, self.ctx._true().into());

        // Track new object
        self.track_new_object(object.clone());

        // The newly object is owned by the box pointer
        let place_state = PlaceState::Own;
        self.exec_state.update_place_state(object, place_state);
    }

    // fn symex_box_from_raw(&mut self, dest: Expr, args: Vec<Expr>) {
    //     let lhs = dest.clone();

    //     let pt = args[0].clone();
    //     // Construct a box pointer
    //     let rhs = self.ctx._box(pt.clone());
    //     self.assign(lhs, rhs, self.ctx._true().into());

    //     // Update place states for objects.
    //     let mut objects = ObjectSet::new();
    //     self.top().cur_state.get_value_set(pt.clone(), &mut objects);
    //     // To make it precisly, if box pointer's value is precisly, make
    //     // the object's place state to be `Own`
    //     if objects.len() == 1 {
    //         for (object, offset) in objects {
    //             if offset != None || object.is_null_object() || object.is_unknown() {
    //                 continue;
    //             }
    //             let root_object = object.extract_root_object();
    //             let symbol = root_object.extract_inner_expr().extract_symbol();
    //             if root_object != object
    //                 || pt.ty().pointee_ty() != root_object.ty()
    //                 || symbol.is_stack_symbol()
    //             {
    //                 continue;
    //             }
    //             // Only update the place in heap. Moreover, only update root object.
    //             let nplace = NPlace(symbol.l1_name());
    //             self.top_mut().cur_state.update_place_state(nplace, PlaceState::Own);
    //         }
    //     }
    // }

    // fn symex_box_into_raw(&mut self, dest: Expr, args: Vec<Expr>) {
    //     let lhs = dest.clone();

    //     let _box = args[0].clone();
    //     // Casting to other pointer
    //     let target_ty = self.ctx.mk_type(lhs.ty());
    //     let rhs = self.ctx.cast(_box.clone(), target_ty);
    //     self.assign(lhs, rhs, self.ctx._true().into());

    //     // Update place states for objects.
    //     let mut objects = ObjectSet::new();
    //     self.top().cur_state.get_value_set(_box.clone(), &mut objects);
    //     // All object should be updated
    //     for (object, offset) in objects {
    //         if offset != None || object.is_null_object() || object.is_unknown() {
    //             continue;
    //         }
    //         let root_object = object.extract_root_object();
    //         let symbol = root_object.extract_inner_expr().extract_symbol();
    //         if root_object != object || symbol.is_stack_symbol() {
    //             continue;
    //         }
    //         // Only update the place in heap.
    //         let nplace = NPlace(symbol.l1_name());
    //         let mut new_place_state = self.top().cur_state.get_place_state(nplace);
    //         new_place_state.meet(PlaceState::Alive);
    //         self.top_mut().cur_state.update_place_state(nplace, new_place_state);
    //     }
    // }
}
