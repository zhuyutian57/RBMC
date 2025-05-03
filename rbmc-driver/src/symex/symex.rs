use stable_mir::CrateDef;
use stable_mir::mir::*;

use super::exec_state::*;
use super::frame::*;
use super::place_state::*;
use crate::config::config::Config;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::program::*;
use crate::symbol::nstring::*;
use crate::symbol::symbol::*;
use crate::vc::vc::*;

pub struct Symex<'cfg> {
    pub(super) config: &'cfg Config,
    pub(super) program: &'cfg Program,
    pub(super) ctx: ExprCtx,
    pub(super) exec_state: ExecutionState<'cfg>,
    pub(super) vc_system: VCSysPtr,
}

impl<'cfg> Symex<'cfg> {
    pub fn new(config: &'cfg Config, vc_system: VCSysPtr) -> Self {
        let ctx = config.expr_ctx.clone();
        let mut exec_state = ExecutionState::new(config, ctx.clone());
        exec_state.setup();

        let mut symex =
            Symex { config, program: &config.program, ctx: ctx.clone(), exec_state, vc_system };
        symex.init();
        symex
    }

    fn init(&mut self) {
        // Init static variable
        for def in self.program.static_variables() {
            let name = NString::from(def.trimmed_name());
            let ty = Type::from(def.ty());
            let symbol = self.exec_state.l0_symbol(name, ty);
            let object = self.ctx.object(symbol.clone());
            self.exec_state.ns.insert_object(object.clone());

            let init_value = match def.eval_initializer() {
                Ok(allocation) => self.make_allocation(&allocation, ty),
                _ => panic!("Some thing wrong?"),
            };
            self.assign(object, init_value, self.ctx._true().into());

            // All static variable is owned by current program
            let mut l1_symbol = symbol;
            self.exec_state.rename(&mut l1_symbol, Level::Level1);
            let nplace = NPlace(l1_symbol.extract_symbol().l1_name());
            self.top_mut().cur_state.update_place_state(nplace, PlaceState::Own);
        }

        // Global varialbes for Encoding
        let alloc_array = self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
        let const_array = self.ctx.constant_array(self.ctx.constant_bool(false), None);
        self.assign(alloc_array, const_array, self.ctx._true().into());
        // Register the initial state
        self.goto(0, self.ctx._true());
    }

    pub fn run(&mut self) {
        while self.exec_state.can_exec() {
            self.symex();
        }
        self.memory_leak_check();
    }

    fn symex(&mut self) {
        while let Some(pc) = self.top_mut().cur_pc() {
            // Merge states
            if self.merge_states(pc) {
                // Couting loop pc
                self.unwind(pc);

                let function_name = self.top().function.name();
                if self.config.enable_display_state()
                    && self.config.enable_display_state_in_function(function_name)
                {
                    println!("Enter {function_name:?} - bb{pc}\n{:?}", self.top().cur_state);
                }

                self.symex_basicblock(pc);
            } else {
                self.top_mut().inc_pc();
            }
        }
        self.symex_end_function();
    }

    pub(super) fn top(&self) -> &Frame<'cfg> {
        self.exec_state.top()
    }

    pub(super) fn top_mut(&mut self) -> &mut Frame<'cfg> {
        self.exec_state.top_mut()
    }

    fn symex_basicblock(&mut self, pc: BasicBlockIdx) {
        let bb = self.top_mut().function.basicblock(pc);
        let function_name = self.top().function.name();
        for (i, statement) in bb.statements.iter().enumerate() {
            self.exec_state.update_span(statement.span);
            self.symex_statement(statement);

            if self.config.enable_display_state_statement()
                && self.config.enable_display_state_in_function(function_name)
            {
                println!(
                    "Symex {function_name:?} bb{pc} statement {i}\n{:?}",
                    self.top().cur_state
                );
            }
        }
        self.exec_state.update_span(bb.terminator.span);
        let is_unwind = self.symex_terminator(&bb.terminator);

        if !is_unwind
            && self.config.enable_display_state_terminator()
            && self.config.enable_display_state_in_function(function_name)
        {
            println!("Symex {function_name:?} bb{pc} terminator\n{:?}", self.top().cur_state);
        }
    }

    fn symex_statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Assign(place, rvalue) => self.symex_assign(place, rvalue),
            StatementKind::StorageLive(local) => self.symex_storagelive(*local),
            StatementKind::StorageDead(local) => self.symex_storagedead(*local),
            _ => {}
        }
    }

    fn symex_storagelive(&mut self, local: Local) {
        // Set a new l1 local variable
        let l1_local = self.exec_state.new_local(local, Level::Level1);
        let l1_num = l1_local.extract_symbol().l1_num();
        self.top_mut().local_states[local] = (l1_num, true);
    }

    fn symex_storagedead(&mut self, local: Local) {
        self.top_mut().local_states[local].1 = false;
    }

    fn symex_terminator(&mut self, terminator: &Terminator) -> bool {
        let mut is_unwind = false;
        match &terminator.kind {
            TerminatorKind::Goto { target } => self.symex_goto(target),
            TerminatorKind::SwitchInt { discr, targets } => self.symex_switchint(discr, targets),
            TerminatorKind::Drop { place, target, .. } => {
                is_unwind = self.symex_drop(place, target);
            }
            TerminatorKind::Call { func, args, destination, target, .. } => {
                is_unwind = self.symex_call(func, args, destination, target);
            }
            TerminatorKind::Return => self.symex_return(),
            TerminatorKind::Assert { cond, expected, msg, target, .. } => {
                self.symex_assert(cond, expected, msg, target)
            }
            _ => {}
        };
        if !is_unwind {
            self.top_mut().inc_pc();
        }
        is_unwind
    }
}
