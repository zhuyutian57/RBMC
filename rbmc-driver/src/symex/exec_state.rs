use std::cell::RefCell;
use std::collections::HashMap;

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
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::*;
use crate::symbol::symbol::*;
use crate::symex::place_state::*;

/// Execution state representing the state of the current program.
pub struct ExecutionState<'cfg> {
    config: &'cfg Config,
    ctx: ExprCtx,
    span: Option<Span>,
    pub(super) ns: Namespace,
    /// The number of frames we have created. Used for variable renaming.
    n: usize,
    /// Current state
    pub(super) cur_state: State,
    frames: Vec<Frame<'cfg>>,
    frame_map: HashMap<usize, usize>,
    pub(super) objects: Vec<Expr>,
    pub(super) renaming: Renaming,
}

impl<'cfg> ExecutionState<'cfg> {
    pub fn new(config: &'cfg Config, ctx: ExprCtx) -> Self {
        ExecutionState {
            config: config,
            ctx: ctx.clone(),
            span: None,
            ns: Namespace::default(),
            n: 0,
            cur_state: State::new(ctx),
            frames: Vec::new(),
            frame_map: HashMap::new(),
            objects: Vec::new(),
            renaming: Renaming::default(),
        }
    }

    pub fn setup(&mut self) {
        // create global variable
        let ty = Type::infinite_array_type(Type::bool_type());
        let ident = Ident::Global(NString::ALLOC_SYM);
        let alloc_array_symbol = self.l0_symbol(ident, ty);
        let alloc_array = self.ctx.object(alloc_array_symbol);
        self.ns.insert_object(alloc_array);
        // Initialized stack
        let entry_function = self.config.program.function_id(self.config.cli.entry_function);
        self.push_frame(entry_function, None, None);
    }

    pub fn can_exec(&self) -> bool {
        !self.frames.is_empty()
    }

    pub fn cur_span(&self) -> Option<Span> {
        self.span
    }

    pub fn update_span(&mut self, span: Span) {
        let name = self.top().function.name();
        if self.config.program.is_local_function(name) {
            self.span = Some(span);
        }
    }

    pub fn top(&self) -> &Frame<'cfg> {
        self.frames.last().expect("Empty frame stack")
    }

    pub fn top_mut(&mut self) -> &mut Frame<'cfg> {
        self.frames.last_mut().expect("Empty frame stack")
    }

    pub fn reset_to_unexplored_state(&mut self) {
        if self.top().unexplored_states.is_empty() {
            panic!("We stuck in a loop, please increase the loop bound");
        }

        self.cur_state.guard.make_false();
        self.top_mut().pc = *self.top().unexplored_states.keys().min().unwrap();
    }

    pub fn push_frame(
        &mut self,
        i: FunctionIdx,
        dest: Option<Place>,
        target: Option<BasicBlockIdx>,
    ) {
        self.n += 1;
        let mut frame =
            Frame::new(self.n, self.config.program.function(i), dest, target);
        self.frames.push(frame);
        self.frame_map.insert(self.n, self.frames.len() - 1);
        // init namspace
        for i in 0..self.top().function.locals().len() {
            self.l0_local(i);
        }
        for &local in self.top().function.locals_alive() {
            self.current_local(local, Level::Level1);
            self.top_mut().local_states[local] = (1, true);
        }
    }

    pub fn pop_frame(&mut self) -> Frame<'cfg> {
        assert!(!self.frames.is_empty());
        self.frames.pop().unwrap()
    }

    pub fn new_object(&mut self, ty: Type) -> Expr {
        let name = NString::from("heap_object_") + self.objects.len().to_string();
        let symbol = Symbol::from(Ident::Heap(name));
        let sym_expr = self.ctx.mk_symbol(symbol, ty);
        // Record the ident
        self.ns.insert_symbol(sym_expr.clone());
        // Create an object not being owned by any variable.
        let object = self.ctx.object(sym_expr);
        self.objects.push(object.clone());
        object
    }

    pub fn l0_symbol(&mut self, ident: Ident, ty: Type) -> Expr {
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
            Level::Level1 => self.renaming.new_l1_symbol(ident),
            Level::Level2 => self.renaming.new_l2_symbol(ident, l1_num),
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
            self.renaming.current_l1_symbol(ident)
        } else {
            self.renaming.current_l2_symbol(ident, 0)
        };
        let ty = self.top().function.local_type(local);
        self.ctx.mk_symbol(symbol, ty)
    }

    pub fn new_local(&mut self, local: Local, level: Level) -> Expr {
        assert!(level != Level::Level0);
        let ident = self.top().local_ident(local);
        let symbol = if level == Level::Level1 {
            self.renaming.new_l1_symbol(ident)
        } else {
            self.renaming.new_l2_symbol(ident, 0)
        };
        let ty = self.top().function.local_type(local);
        self.ctx.mk_symbol(symbol, ty)
    }

    pub fn rename(&mut self, expr: &mut Expr, level: Level) {
        match level {
            Level::Level0 => return,
            Level::Level1 => self.renaming.l1_rename(expr),
            Level::Level2 => self.renaming.l2_rename(expr, true),
        };
    }

    fn constant_propagate(&mut self, lhs: Expr, rhs: Expr) {
        if rhs.is_object() {
            self.constant_propagate(lhs, rhs.extract_inner_expr());
            return;
        }

        if !self.is_constant_value(rhs.clone()) {
            self.renaming.constant_propagate(lhs, None);
            return;
        }

        assert!(lhs.is_symbol());
        self.renaming.constant_propagate(lhs, Some(rhs));
    }

    fn is_constant_value(&self, expr: Expr) -> bool {
        if expr.is_constant() || expr.is_type() {
            return true;
        }

        if expr.is_aggregate() {
            return expr
                .extract_fields()
                .iter()
                .fold(true, |acc, field| acc && self.is_constant_value(field.clone()));
        }

        if expr.is_binary() {
            let lhs = expr.extract_lhs();
            let rhs = expr.extract_rhs();
            return self.is_constant_value(lhs) && self.is_constant_value(rhs);
        }

        if expr.is_unary() {
            return self.is_constant_value(expr.extract_inner_expr());
        }

        if expr.is_cast() {
            return self.is_constant_value(expr.extract_src());
        }

        // Address is fixed when the memory is alloced
        if expr.is_address_of() {
            return self.is_constant_address(expr.extract_object());
        }

        if expr.is_pointer() {
            let base = expr.extract_pointer_address();
            let meta = expr.extract_pointer_meta();
            return self.is_constant_value(base) && self.is_constant_value(meta);
        }

        if expr.is_pointer_base() || expr.is_pointer_meta() {
            return self.is_constant_value(expr.extract_inner_pointer());
        }

        if expr.is_variant() {
            return self.is_constant_value(expr.extract_variant_data());
        }

        if expr.is_as_variant() {
            return self.is_constant_value(expr.extract_enum());
        }

        if expr.is_match_variant() {
            return self.is_constant_value(expr.extract_enum());
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

    fn ith_frame(&self, i: usize) -> Option<&Frame<'cfg>> {
        let (mut l, mut r) = (0, self.frames.len());
        while r - l > 1 {
            let m = (l + r) / 2;
            let id = self.frames[m].id;
            if i < id {
                r = m;
            } else {
                l = m;
            }
        }
        if l == r { None } else { Some(&self.frames[l]) }
    }

    pub fn get_place_state(&self, place: &Expr) -> PlaceState {
        if place.is_object() {
            return self.get_place_state(&place.extract_inner_expr());
        }

        assert!(place.is_symbol());
        let symbol = place.extract_symbol();
        if symbol.is_global_symbol() {
            PlaceState::Own
        } else if symbol.is_stack_symbol() {
            let frame = self.ith_frame(symbol.frame_id());
            match frame {
                Some(x) => {
                    if x.id == symbol.frame_id() {
                        x.get_local_place_state(symbol)
                    } else {
                        PlaceState::Dead
                    }
                }
                _ => PlaceState::Dead,
            }
        } else {
            self.cur_state.get_place_state(NPlace(symbol.l1_name()))
        }
    }

    pub fn update_place_state(&mut self, place: Expr, state: PlaceState) {
        if place.is_symbol() {
            let mut l1_place = place;
            self.rename(&mut l1_place, Level::Level1);
            let symbol = l1_place.extract_symbol();
            assert!(symbol.is_heap_symbol());
            let nplace = NPlace(symbol.l1_name());
            self.cur_state.update_place_state(nplace, state);
            return;
        }

        if place.is_object() {
            let inner_object = place.extract_inner_expr();
            self.update_place_state(inner_object, state);
            return;
        }

        panic!("Do not support place state: {place:?}");
    }

    pub fn assignment(&mut self, lhs: Expr, rhs: Expr) {
        assert!(lhs.is_symbol() && !lhs.extract_symbol().is_level2());

        // Constant propagation
        self.constant_propagate(lhs.clone(), rhs.clone());

        // `Layout` is only used for allocation
        if rhs.is_type() {
            return;
        }

        // Update value Set
        self.assign_value_set(lhs, rhs);
    }

    fn assign_value_set(&mut self, lhs: Expr, rhs: Expr) {
        if !lhs.ty().contains_ptr_field() { return; }

        if lhs.ty().is_struct() || lhs.ty().is_tuple() {
            // Update for each field
            let ty = lhs.ty();
            let lhs_object = self.ctx.object(lhs.clone());
            let rhs_object = self.ctx.object(rhs.clone());
            for i in 0..ty.fields() {
                let fty = ty.field_type(i);
                if ty.field_type(i).is_zero_sized_type() {
                    continue;
                }
                let idx = self.ctx.constant_usize(i);
                let lhs_field = self.ctx.index(lhs_object.clone(), idx.clone(), fty);
                let rhs_field = self.ctx.index(rhs_object.clone(), idx.clone(), fty);
                self.assign_value_set(lhs_field, rhs_field);
            }
            return;
        }

        if lhs.is_index() {
            // Identifier form: `<l1_name>.<field_id/field_name>`
            assert!(lhs.extract_index().is_constant());
            // Use global as a wrapper.
            let ident = Ident::Global(NString::from(format!("{lhs:?}")));
            let name = Symbol::from(ident);
            let new_lhs = self.ctx.mk_symbol(name, lhs.ty());
            let mut new_rhs = rhs.clone();
            new_rhs.simplify();
            self.assign_value_set(new_lhs, new_rhs);
            return;
        }
        
        assert!(lhs.is_symbol());

        // For enum, we flat all fields of all variants in value set.
        // For example, `enum Node { A, B(i32), C(u8, u8) }` has three fields.
        // A variable `x` of type `Node` has three fields of form `x.0[<variant_idx>-<field>]`,
        // in value set, e.g. `x.0[1-0]`, `x.0[2-0]` and `x.0[2-1]`, where `0` denote the data field.
        if lhs.ty().is_enum() {
            // Remove all possible fields firstly.
            let prefix = NString::from(format!("{lhs:?}.0"));
            self.cur_state.remove_pointer_with_prefix(prefix);
            // Do assignment
            if rhs.is_variant() || rhs.is_constant() {
                let (data, i) = if rhs.is_variant() {
                    (rhs.extract_variant_data(), rhs.extract_variant_idx())
                } else {
                    let args = rhs.extract_constant().to_adt().0;
                    let i = bigint_to_usize(&args[0].to_integer());
                    let ty = lhs.ty().enum_variant_data_type(i);
                    if ty.is_zero_sized_type() {
                        (self.ctx.constant_zst(ty), i)
                    } else {
                        (self.ctx.constant(args[1].clone(), ty), i)
                    }
                };
                if data.ty().is_zero_sized_type() { return; }
                let rhs_object = self.ctx.object(data.clone());
                for j in 0..data.ty().fields() {
                    let ident = Ident::Global(NString::from(format!("{lhs:?}.0[{i}-{j}]")));
                    let fty = data.ty().field_type(j);
                    if fty.is_zero_sized_type() { continue; }
                    let new_lhs = self.ctx.mk_symbol(ident.into(), fty);
                    let mut new_rhs = self.ctx.index(
                        rhs_object.clone(),
                        self.ctx.constant_usize(j),
                        fty
                    );
                    new_rhs.simplify();
                    self.assign_value_set(new_lhs, new_rhs);
                }
            } else {
                for i in 0..lhs.ty().enum_variants() {
                    let data_ty = lhs.ty().enum_variant_data_type(i);
                    if data_ty.is_zero_sized_type() { continue; }
                    let rhs_object = self.ctx.object(rhs.clone());
                    for j in 0..data_ty.fields() {
                        let ident = Ident::Global(NString::from(format!("{lhs:?}.0[{i}-{j}]")));
                        let fty = data_ty.field_type(j);
                        if fty.is_zero_sized_type() { continue; }
                        let new_lhs = self.ctx.mk_symbol(ident.into(), fty);
                        let mut new_rhs = self.ctx.index(
                            rhs_object.clone(),
                            self.ctx.constant_usize(j),
                            fty
                        );
                        new_rhs.simplify();
                        self.assign_value_set(new_lhs, new_rhs);
                    }
                }
            }
            return;
        }

        assert!(lhs.ty().is_primitive_ptr());
        self.assignment_value_set(lhs, rhs);
    }

    fn assignment_value_set(&mut self, lhs: Expr, rhs: Expr) {
        let l1_lhs = lhs.clone();
        let mut l1_rhs = rhs.clone();
        // lhs is already in level1
        self.rename(&mut l1_rhs, Level::Level1);
        let mut objects = ObjectSet::new();
        self.cur_state.get_value_set(l1_rhs.clone(), &mut objects);
        self.cur_state.assign(l1_lhs.clone(), objects);

        // Cache local pointers
        let pt = l1_lhs.extract_symbol().name();
        if pt.starts_with(self.top().frame_ident()) {
            self.top_mut().local_pointers.insert(pt);
        }
    }
}
