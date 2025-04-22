use stable_mir::mir::mono::Instance;
use stable_mir::CrateDef;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::symex::*;

/// This mod defines symbolic execution of api in core
/// 
/// TODO: seperate to multiple crate
impl<'cfg> Symex<'cfg> {
    pub fn symex_core_api(&mut self, instance: Instance, args: Vec<Expr>, dest: Expr) {
        let fty = Type::from(instance.ty());
        let name = NString::from(fty.fn_def().0.trimmed_name());
        if name == "slice_index_order_fail"
            || name == "slice_start_index_len_fail"
            || name == "slice_end_index_len_fail" {
            self.symex_slice_assertion(name);
        } else {
            panic!("Not support {name:?}");
        }
    }

    fn symex_slice_assertion(&mut self, fname: NString) {
        let msg = NString::from("Slice failt: ") +
            fname.to_string().replace('_', " ");
        self.claim(msg, self.ctx._true());
    }
}