use std::cell::RefCell;

use num_bigint::BigInt;
use stable_mir::CrateDef;
use stable_mir::mir::*;

use super::frame::*;
use super::namespace::Namespace;
use super::renaming::*;
use super::state::*;
use super::value_set::ObjectSet;
use crate::expr::context::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::program::function::FunctionIdx;
use crate::program::program::*;
use crate::symbol::nstring::*;
use crate::symbol::symbol::*;
use crate::symex::place_state::*;

/// Execution state representing the state of the current program.
/// Multi-thread program is not supported yet.
///
/// Moreover, `func_cnt` is used for identifying each function. It
/// is used for naming variables later.
pub struct ExecutionState<'cfg> {
    program: &'cfg Program,
    ctx: ExprCtx,
    pub(super) ns: Namespace,
    pub(super) objects: Vec<Expr>,
    func_cnt: Vec<usize>,
    frames: Vec<Frame<'cfg>>,
    pub(super) renaming: RefCell<Renaming>,
}

impl<'cfg> ExecutionState<'cfg> {
    pub fn new(program: &'cfg Program, ctx: ExprCtx) -> Self {
        ExecutionState {
            program,
            ctx,
            ns: Namespace::default(),
            objects: Vec::new(),
            func_cnt: vec![0; program.size()],
            frames: Vec::new(),
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
        let symbol = Symbol::new(name, 0, 0, Level::Level0);
        let sym_expr = self.ctx.mk_symbol(symbol, ty);
        // Record the ident
        self.ns.insert_symbol(sym_expr.clone());
        // Create an object not being owned by any variable.
        let object = self.ctx.object(sym_expr);
        self.objects.push(object.clone());
        object
    }

    pub fn cur_state(&self) -> &State {
        self.top().cur_state()
    }

    pub fn cur_state_mut(&mut self) -> &mut State {
        self.top_mut().cur_state_mut()
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
            self.ctx.clone(),
            self.func_cnt[i],
            self.program.function(i),
            destination,
            target,
        );
        if !self.frames.is_empty() {
            frame.cur_state = self.cur_state().clone();
        }
        self.frames.push(frame);
        // init namspace
        for i in 0..self.top().function().locals().len() {
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
            let symbol = Symbol::new(ident, 0, 0, Level::Level0);
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
        let ty = self.top().function().local_type(local);
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
        let ty = self.top().function().local_type(local);
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
        let ty = self.top().function().local_type(local);
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
        if rhs.is_cast() {
            self.constant_propagate(lhs, rhs.extract_src());
            return;
        }

        if rhs.is_object() {
            self.constant_propagate(lhs, rhs.extract_inner_expr());
            return;
        }

        if !rhs.is_constant() && !rhs.is_type() {
            return;
        }
        assert!(lhs.is_symbol());
        self.renaming.borrow_mut().constant_propagate(lhs, rhs);
    }

    fn get_place_state_for_stack_symbol(&self, ident: NString) -> PlaceState {
        // Static variables
        for x in self.program.static_variables() {
            if ident == NString::from(x.trimmed_name()) {
                return PlaceState::Own;
            }
        }
        // Local without storagelive
        assert!(ident.contains("::".into()));
        let func = ident.sub_str(0, ident.find(":".into()).unwrap());
        for frame in self.frames.iter().rev() {
            if func == frame.function_id() {
                return PlaceState::Own;
            }
        }
        PlaceState::Dead
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
        if state.is_unknown() && symbol.is_stack_symbol() {
            return self.get_place_state_for_stack_symbol(symbol.ident());
        }
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

    pub fn assign(&mut self, lhs: Expr, rhs: Expr) {
        assert!(lhs.is_symbol());

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
                let i = self.ctx.constant_isize(BigInt::from(i));
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
