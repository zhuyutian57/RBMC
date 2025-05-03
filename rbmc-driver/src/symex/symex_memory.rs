use super::symex::*;
use crate::expr::expr::*;
use crate::symbol::nstring::*;
use crate::symbol::symbol::Ident;

impl<'cfg> Symex<'cfg> {
    pub(super) fn track_new_object(&mut self, object: Expr) {
        assert!(object.is_object());
        // alloc[&object] = true
        let ident = Ident::Global(NString::ALLOC_SYM);
        let alloc_array = self.exec_state.ns.lookup_object(ident);
        let address = self.ctx.address_of(object.clone(), object.extract_address_type());
        let store = self.ctx.store(alloc_array.clone(), address, self.ctx._true());
        self.assign(alloc_array, store, self.ctx._true().into());
    }
}
