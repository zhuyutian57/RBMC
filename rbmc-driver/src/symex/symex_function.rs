use stable_mir::mir::mono::Instance;
use stable_mir::mir::*;

use super::place_state::NPlace;
use super::place_state::PlaceState;
use super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::symbol::nstring::NString;
use crate::symbol::symbol::Ident;
use crate::symbol::symbol::Level;
use crate::symbol::symbol::Symbol;

impl<'cfg> Symex<'cfg> {
    pub(super) fn symex_call(
        &mut self,
        func: &Operand,
        args: &Vec<Operand>,
        dest: &Place,
        target: &Option<BasicBlockIdx>,
    ) -> bool {
        let instance = self.top_mut().function.operand_type(func).function_instance();
        let ty = Type::from(instance.ty());

        let args_exprs = args.iter().map(|x| self.make_operand(x)).collect::<Vec<_>>();

        let is_rbmc_nondet = ty.is_rbmc_nondet();
        let is_rust_builtin = ty.is_rust_builtin_function();

        if is_rbmc_nondet {
            self.symex_nondet(dest);
        } else if is_rust_builtin {
            self.symex_rust_builtin_function(instance, args_exprs, dest);
        } else {
            // Unwinding function
            self.symex_function(instance, args_exprs, Some(dest.clone()), target);
        }

        if is_rbmc_nondet || is_rust_builtin {
            self.top_mut().pc += 1;
        }
        
        !is_rbmc_nondet && !is_rust_builtin
    }

    fn symex_nondet(&mut self, dest: &Place) {
        let lhs = self.make_project(dest);
        let n = self.exec_state.ns.lookup_nondet_count(lhs.ty());
        let ident = Ident::Global(NString::from(format!("nondet_{:?}_{n}", lhs.ty())));
        let symbol = Symbol::from(ident);
        let nondet = self.ctx.mk_symbol(symbol, lhs.ty());
        self.assign(lhs, nondet, self.ctx._true().into());
    }

    fn symex_rust_builtin_function(&mut self, instance: Instance, args: Vec<Expr>, dest: &Place) {
        let name = NString::from(instance.name());
        let ret = self.make_project(dest);
        if name.starts_with("std".into()) {
            self.symex_std_api(instance, args, ret);
        } else if name.starts_with("core".into()) {
            self.symex_core_api(instance, args, ret);
        } else {
            panic!("Do not support {name:?}")
        }
    }

    pub fn symex_function(
        &mut self,
        instance: Instance,
        args: Vec<Expr>,
        dest: Option<Place>,
        target: &Option<BasicBlockIdx>,
    ) {
        let i = self.program.function_id(instance.trimmed_name().into());
        self.exec_state.push_frame(i, dest, *target);
        // Set arguements
        let parameters = self.top_mut().function.args();
        if !parameters.is_empty() {
            for &i in parameters.iter() {
                let lhs = self.exec_state.l0_local(i);
                let rhs = args[i - 1].clone();
                self.assign(lhs, rhs, self.ctx._true().into());
            }
        }
    }

    pub(super) fn symex_return(&mut self) {
        let frame = self.exec_state.pop_frame();
        if !self.exec_state.can_exec() {
            return;
        }

        // Assign return value
        if !frame.function.local_type(0).is_unit() {
            if let Some(ret) = &frame.dest {
                let lhs = self.make_project(ret);
                let rhs_ident = frame.local_ident(0);
                let rhs_ty = frame.function.local_type(0);
                let rhs = self.exec_state.l0_symbol(rhs_ident, rhs_ty);
                self.assign(lhs, rhs.clone(), self.ctx._true().into());
                // Symex semantic thats relates to place state
                // TODO: support more situations
                let fty = frame.function.ty();
                if fty.is_function_with_special_semantic() {
                    let def = frame.function.ty().fn_def().0;
                    let mut ret = rhs;
                    self.exec_state.rename(&mut ret, Level::Level1);
                    self.symex_special_semantic(def, ret);
                }
            }
        }

        // Clear namspace.
        self.exec_state.ns.clear_symbols_with_prefix(frame.frame_ident());

        // Clear renaming.
        self.exec_state.renaming.cleanr_locals(frame.frame_ident());

        // Clear place state set and value set.
        self.exec_state.cur_state.remove_stack_places(frame.frame_ident());

        // Recover pc.
        if let Some(t) = frame.target {
            self.symex_goto(t);
        } else {
            self.exec_state.reset_to_unexplored_state();
        }

        // Display state after returning function
        let function_name = self.top().function.name();
        if self.config.enable_display_state_terminator()
            && self.config.enable_display_state_in_function(function_name)
        {
            println!(
                "Symex {function_name:?} bb{} terminator\n{:?}",
                self.top().pc,
                self.exec_state.cur_state
            );
        }
    }
}
