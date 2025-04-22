pub(super) mod symex_alloc;
pub(super) mod symex_boxed;
pub(super) mod symex_ops;
pub(super) mod symex_ptr;
pub(super) mod symex_vec;

use stable_mir::mir::mono::Instance;
use stable_mir::CrateDef;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::place_state::PlaceState;
use crate::symex::projection::Mode;
use crate::symex::symex::*;

use symex_alloc::*;
use symex_boxed::*;

/// This mod defines symbolic execution of api in std
impl<'cfg> Symex<'cfg> {
    pub fn symex_std_api(&mut self, instance: Instance, args: Vec<Expr>, dest: Expr) {
        let fty = Type::from(instance.ty());
        let name = NString::from(fty.fn_def().0.trimmed_name());
        if name.starts_with("std::alloc".into()) {
            self.symex_alloc_api(instance, args, dest);
        } else if name.starts_with("std::boxed".into()) {
            self.symex_boxed_api(instance, args, dest);
        } else {
            panic!("Not support {name:?}");
        }
    }
}
