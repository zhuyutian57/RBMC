use super::symex::*;
use crate::expr::expr::*;
use crate::symbol::nstring::*;

impl<'cfg> Symex<'cfg> {
    pub(super) fn track_new_object(&mut self, object: Expr) {
        assert!(object.is_object());
        let ctx = object.ctx.clone();

        // alloc[&object] = true
        let alloc_array = self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
        let pointer_base =
            ctx.pointer_base(ctx.address_of(object.clone(), object.extract_address_type()));
        let store = ctx.store(alloc_array.clone(), pointer_base, ctx._true());
        self.assign(alloc_array, store, self.ctx._true().into());
    }
}
