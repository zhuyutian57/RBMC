use super::symex::Symex;

impl<'cfg> Symex<'cfg> {
    // pub(super) fn symex_builtin_function(
    //     &mut self,
    //     fndef: &FunctionDef,
    //     args: Vec<Expr>,
    //     dest: Expr,
    // ) {
    //     let name = NString::from(fndef.0.trimmed_name());
    //     if name == "nondet" {
    //         self.symex_nondet(dest);
    //     } else {
    //         panic!("Not support for {name:?}");
    //     }
    // }

    // fn symex_nondet(&mut self, dest: Expr) {
    //     let lhs = dest.clone();
    //     let n = self.exec_state.ns.lookup_nondet_count(lhs.ty());
    //     let name = NString::from(format!("nondet_{:?}_{n}", lhs.ty()));
    //     let symbol = Symbol::from(name);
    //     let nondet = self.ctx.mk_symbol(symbol, lhs.ty());
    //     self.assign(lhs, nondet, self.ctx._true().into());
    // }
}
