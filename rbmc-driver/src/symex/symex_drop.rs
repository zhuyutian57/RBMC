use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) {
        // Drop recursively
        let place = self.make_project(place);
        let drop_instance = place.ty().drop_instance();
        let object = if place.is_object() { place } else { self.ctx.object(place) };
        let address =
            self.ctx.address_of(object.clone(), Type::ptr_type(object.ty(), Mutability::Mut));
        self.symex_function(drop_instance, vec![address], None, &Some(*target));
    }
}
