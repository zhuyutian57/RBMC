use stable_mir::mir::*;

use super::symex::*;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::*;
use crate::symbol::nstring::NString;
use crate::symex::projection::Mode;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_drop(&mut self, place: &Place, target: &BasicBlockIdx) -> bool {
        // Drop recursively
        let place = self.make_project(place);
        if place.ty().is_bool() || place.ty().is_integer() { return false; }
        let drop_instance = place.ty().drop_instance();
        let address = self.ctx.address_of(
            self.ctx.object(place.clone()),
            Type::ptr_type(place.ty(), Mutability::Mut)
        );
        self.symex_function(drop_instance, vec![address], None, &Some(*target));
        true
    }
}
