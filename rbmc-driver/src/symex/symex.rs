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
use crate::solvers::solver::Solver;
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

        let mut symex = Symex {
            config,
            program: &config.program,
            ctx: ctx.clone(),
            exec_state,
            vc_system
        };
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
                if self.config.cli.enable_display_state_bb() &&
                    self.config.program.is_local_function(self.top().function.name()) {
                    println!(
                        "Enter {:?} - bb{pc}\n{:?}",
                        self.top().function.name(),
                        self.top().cur_state
                    );

                    self.exec_state.renaming.borrow().constant_map.iter().for_each(
                        |(s, v)|
                        if s.name().starts_with("pc".into()) {
                            println!("{s:?} = {v:?}");
                        }
                    );
                }
                let bb = self.top_mut().function.basicblock(pc);
                self.symex_basicblock(bb);
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

    fn symex_basicblock(&mut self, bb: &BasicBlock) {
        for (i, statement) in bb.statements.iter().enumerate() {
            self.exec_state.update_span(statement.span);
            self.symex_statement(statement);
            if self.config.cli.enable_display_state_statement() &&
                self.config.program.is_local_function(self.top().function.name()) {
                println!("After symex {i}\n{:?}", self.top_mut().cur_state);
            }
        }
        self.exec_state.update_span(bb.terminator.span);
        self.symex_terminator(&bb.terminator);
        if self.config.cli.enable_display_state_terminator() &&
            self.config.program.is_local_function(self.top().function.name()) {
            println!("After symex terminator\n{:?}", self.top_mut().cur_state);
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
        let nplace = NPlace(l1_local.extract_symbol().l1_name());
        self.top_mut().cur_state.update_place_state(nplace, PlaceState::Own);
    }

    fn symex_storagedead(&mut self, local: Local) {
        let l1_local = self.exec_state.current_local(local, Level::Level1);
        if l1_local.ty().is_any_ptr() {
            self.top_mut().cur_state.remove_pointer(l1_local.clone());
        }
        let nplace = NPlace(l1_local.extract_symbol().l1_name());
        // Just remove to safe memory
        self.top_mut().cur_state.remove_place(nplace);
    }

    fn symex_terminator(&mut self, terminator: &Terminator) {
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
    }
}