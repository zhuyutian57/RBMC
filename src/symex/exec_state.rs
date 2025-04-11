use std::cell::RefCell;

use stable_mir::mir::*;
use stable_mir::ty::Span;

use super::frame::*;
use super::namespace::Namespace;
use super::renaming::*;
use super::state::*;
use super::value_set::ObjectSet;
use crate::config::config::Config;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::function::FunctionIdx;
use crate::symbol::nstring::*;
use crate::symbol::symbol::*;
use crate::symex::place_state::*;

/// Execution state representing the state of the current program.
/// Multi-thread program is not supported yet.
///
/// Moreover, `func_cnt` is used for identifying each function. It
/// is used for naming variables later.
pub struct ExecutionState<'cfg> {
    config: &'cfg Config,
    ctx: ExprCtx,
    pub(super) span: Option<Span>,
    pub(super) ns: Namespace,
    func_cnt: Vec<usize>,
    frames: Vec<Frame<'cfg>>,
    pub(super) objects: Vec<Expr>,
    pub(super) renaming: RefCell<Renaming>,
}

impl<'cfg> ExecutionState<'cfg> {
    pub fn new(config: &'cfg Config, ctx: ExprCtx) -> Self {
        ExecutionState {
            config: config,
            ctx,
            span: None,
            ns: Namespace::default(),
            func_cnt: vec![0; config.program.size()],
            frames: Vec::new(),
            objects: Vec::new(),
            renaming: RefCell::new(Renaming::default()),
        }
    }

    pub fn setup(&mut self) {
        // create global variable
        let ty = Type::infinite_array_type(Type::bool_type());
        let alloc_array_symbol = self.l0_symbol(NString::ALLOC_SYM, ty);
        let alloc_array = self.ctx.object(alloc_array_symbol);
        self.ns.insert_object(alloc_array);
        // Initialized stack
        self.push_frame(0, None, None);
    }

    pub fn can_exec(&self) -> bool {
        self.frames.len() > 1 || self.frames.len() == 1 && self.top().cur_pc() != None
    }

    pub fn new_object(&mut self, ty: Type) -> Expr {
        let name = NString::from("heap_object_") + self.objects.len().to_string();
        let symbol = Symbol::from(name);
        let sym_expr = self.ctx.mk_symbol(symbol, ty);
        // Record the ident
        self.ns.insert_symbol(sym_expr.clone());
        // Create an object not being owned by any variable.
        let object = self.ctx.object(sym_expr);
        self.objects.push(object.clone());
        object
    }

    pub fn cur_state(&self) -> &State {
        &self.top().cur_state
    }

    pub fn cur_state_mut(&mut self) -> &mut State {
        &mut self.top_mut().cur_state
    }

    pub fn top(&self) -> &Frame<'cfg> {
        self.frames.last().expect("Empty frame stack")
    }

    pub fn top_mut(&mut self) -> &mut Frame<'cfg> {
        self.frames.last_mut().expect("Empty frame stack")
    }

    pub fn push_frame(
        &mut self,
        i: FunctionIdx,
        destination: Option<Place>,
        target: Option<BasicBlockIdx>,
    ) {
        self.func_cnt[i] += 1;
        let mut frame = Frame::new(
            self.config,
            self.func_cnt[i],
            self.config.program.function(i),
            destination,
            target,
        );
        if !self.frames.is_empty() {
            frame.cur_state = self.cur_state().clone();
        }
        self.frames.push(frame);
        // init namspace
        for i in 0..self.top().function.locals().len() {
            self.l0_local(i);
        }
    }

    pub fn pop_frame(&mut self) -> Frame<'cfg> {
        assert!(!self.frames.is_empty());
        self.frames.pop().unwrap()
    }

    pub fn l0_symbol(&mut self, ident: NString, ty: Type) -> Expr {
        if self.ns.containts_symbol(ident) {
            self.ns.lookup_symbol(ident)
        } else {
            let symbol = Symbol::from(ident);
            let symbol_expr = self.ctx.mk_symbol(symbol, ty);
            self.ns.insert_symbol(symbol_expr.clone());
            symbol_expr
        }
    }

    pub fn new_symbol(&mut self, symbol: &Expr, level: Level) -> Expr {
        assert!(symbol.is_symbol() && level != Level::Level0);
        let sym = symbol.extract_symbol();
        let ident = sym.ident();
        let l1_num = sym.l1_num();
        let new_sym = match level {
            Level::Level1 => self.renaming.borrow_mut().new_l1_symbol(ident),
            Level::Level2 => self.renaming.borrow_mut().new_l2_symbol(ident, l1_num),
            _ => panic!(),
        };
        self.ctx.mk_symbol(new_sym, symbol.ty())
    }

    pub fn l0_local(&mut self, local: Local) -> Expr {
        let ident = self.top().local_ident(local);
        let ty = self.top().function.local_type(local);
        self.l0_symbol(ident, ty)
    }

    pub fn current_local(&mut self, local: Local, level: Level) -> Expr {
        assert!(level != Level::Level0);
        let ident = self.top().local_ident(local);
        let symbol = if level == Level::Level1 {
            self.renaming.borrow_mut().current_l1_symbol(ident)
        } else {
            self.renaming.borrow_mut().current_l2_symbol(ident, 0)
        };
        let ty = self.top().function.local_type(local);
        self.ctx.mk_symbol(symbol, ty)
    }

    pub fn new_local(&mut self, local: Local, level: Level) -> Expr {
        assert!(level != Level::Level0);
        let ident = self.top().local_ident(local);
        let symbol = if level == Level::Level1 {
            self.renaming.borrow_mut().new_l1_symbol(ident)
        } else {
            self.renaming.borrow_mut().new_l2_symbol(ident, 0)
        };
        let ty = self.top().function.local_type(local);
        self.ctx.mk_symbol(symbol, ty)
    }

    pub fn rename(&self, expr: &mut Expr, level: Level) {
        match level {
            Level::Level0 => return,
            Level::Level1 => self.renaming.borrow_mut().l1_rename(expr),
            Level::Level2 => self.renaming.borrow_mut().l2_rename(expr, true),
        };
    }

    fn constant_propagate(&mut self, lhs: Expr, rhs: Expr) {
        if rhs.is_object() {
            self.constant_propagate(lhs, rhs.extract_inner_expr());
            return;
        }

        if !self.is_constant_value(rhs.clone()) {
            self.renaming.borrow_mut().constant_propagate(lhs, None);
            return;
        }
        assert!(lhs.is_symbol());
        self.renaming.borrow_mut().constant_propagate(lhs, Some(rhs));
    }

    fn is_constant_value(&self, expr: Expr) -> bool {
        if expr.is_constant() || expr.is_type() {
            return true;
        }
        if expr.is_cast() {
            return self.is_constant_value(expr.extract_src());
        }
        // Address is fixed when the memory is alloced
        if expr.is_address_of() {
            return self.is_constant_address(expr.extract_object());
        }
        false
    }

    fn is_constant_address(&self, expr: Expr) -> bool {
        if expr.is_symbol() {
            return true;
        }

        if expr.is_object() {
            return self.is_constant_address(expr.extract_inner_expr());
        }

        if expr.is_index() {
            let inner_object = expr.extract_object();
            let index = expr.extract_index();
            return self.is_constant_address(inner_object) && self.is_constant_value(index);
        }

        false
    }

    pub fn get_place_state(&self, place: &Expr) -> PlaceState {
        if place.is_object() {
            return self.get_place_state(&place.extract_inner_expr());
        }

        assert!(place.is_symbol());
        let symbol = place.extract_symbol();
        let l1_name = symbol.l1_name();
        let nplace = NPlace(l1_name);
        let state = self.top().cur_state.get_place_state(nplace);
        state
    }

    pub fn update_place_state(&mut self, place: Expr, state: PlaceState) {
        if place.is_symbol() {
            let mut l1_place = place;
            self.rename(&mut l1_place, Level::Level1);
            let symbol = l1_place.extract_symbol();
            assert!(symbol.ident().contains("heap_object".into()));
            let nplace = NPlace(symbol.l1_name());
            self.cur_state_mut().update_place_state(nplace, state);
            return;
        }

        if place.is_object() {
            let inner_object = place.extract_inner_expr();
            self.update_place_state(inner_object, state);
            return;
        }

        panic!("Do not support place state: {place:?}");
    }

    pub fn assignment(&mut self, mut lhs: Expr, rhs: Expr) {
        assert!(lhs.is_symbol() && !lhs.extract_symbol().is_level2());

        if lhs.extract_symbol().is_level0() {
            self.rename(&mut lhs, Level::Level1);
        }

        // Constant propagation
        self.constant_propagate(lhs.clone(), rhs.clone());

        // `Layout` is only used for allocation
        if rhs.is_type() {
            return;
        }

        // Update value Set
        self.update_value_set_rec(lhs, rhs);
    }

    fn update_value_set_rec(&mut self, lhs: Expr, rhs: Expr) {
        if lhs.ty().is_any_ptr() {
            let mut l1_lhs = lhs.clone();
            let mut l1_rhs = rhs.clone();
            self.rename(&mut l1_lhs, Level::Level1);
            self.rename(&mut l1_rhs, Level::Level1);
            let mut objects = ObjectSet::new();
            self.cur_state().get_value_set(l1_rhs.clone(), &mut objects);
            self.cur_state_mut().assign(l1_lhs, objects);
            return;
        }

        if lhs.ty().is_struct() {
            // We do not care the ownership here
            let lhs_object = self.ctx.object(lhs.clone());
            let rhs_object =
                if rhs.is_object() { rhs.clone() } else { self.ctx.object(rhs.clone()) };
            for (i, (_, ty)) in lhs.ty().struct_def().1.iter().enumerate() {
                if !ty.is_any_ptr() {
                    return;
                }
                let i = self.ctx.constant_isize(i as isize);
                let new_lhs = self.ctx.index(lhs_object.clone(), i.clone(), *ty);
                let new_rhs = self.ctx.index(rhs_object.clone(), i.clone(), *ty);
                self.update_value_set_rec(new_lhs, new_rhs);
            }
            return;
        }

        if lhs.ty().is_array() {
            if lhs.ty().elem_type().is_any_ptr() {
                // TODO
            }
            return;
        }
    }
}
