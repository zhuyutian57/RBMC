use num_bigint::BigInt;
use stable_mir::CrateDef;
use stable_mir::abi::FieldsShape;
use stable_mir::mir::Operand;
use stable_mir::mir::Place;
use stable_mir::mir::alloc::GlobalAlloc;
use stable_mir::target::*;
use stable_mir::ty::*;

use super::projection::*;
use super::state::State;
use super::symex::*;
use crate::expr::constant::*;
use crate::expr::expr::*;
use crate::expr::guard::*;
use crate::expr::ty::*;
use crate::program::function::*;
use crate::program::program::*;
use crate::symbol::nstring::*;
use crate::symbol::symbol::*;

impl<'cfg> Symex<'cfg> {
    pub(super) fn merge_states(&mut self, pc: Pc) -> bool {
        let mut state_vec = self.top_mut().unexplored_states_from(pc);

        if let Some(states) = state_vec.as_mut() {
            for state in states {
                if state.guard.is_false() {
                    continue;
                }

                // SSA assigment
                self.phi_function(state);

                self.exec_state.cur_state.merge(state);
            }
        }

        !self.exec_state.cur_state.guard.is_false()
    }

    fn phi_function(&mut self, nstate: &mut State) {
        if let None = nstate.renaming {
            return;
        }

        let new_guard = nstate.guard.clone() - self.exec_state.cur_state.guard.clone();

        let mut nrenaming = nstate.renaming.as_mut().unwrap();
        for ident in nrenaming.variables() {
            let symbol = nrenaming.current_l1_symbol(ident);
            let l1_ident = (symbol.ident(), symbol.l1_num());
            let cur_l2_num = self.exec_state.renaming.l2_count(l1_ident);
            let n_l2_num = nrenaming.l2_count(l1_ident);

            if cur_l2_num == n_l2_num || n_l2_num == 0 {
                continue;
            }

            let mut cur_rhs = self.exec_state.ns.lookup_symbol(ident);
            let mut new_rhs = self.exec_state.ns.lookup_symbol(ident);

            // Get l1 number
            nrenaming.l1_rename(&mut cur_rhs);
            nrenaming.l1_rename(&mut new_rhs);

            // Current assignment
            self.exec_state.renaming.l2_rename(&mut cur_rhs, true);
            // Other assignment
            nrenaming.l2_rename(&mut new_rhs, true);

            let rhs = if self.exec_state.cur_state.guard.is_false() {
                new_rhs
            } else {
                self.ctx.ite(new_guard.to_expr(), new_rhs, cur_rhs)
            };

            let mut lhs = self.exec_state.ns.lookup_symbol(ident);

            // Rename to l1_lhs
            self.exec_state.rename(&mut lhs, Level::Level1);
            self.exec_state.assignment(lhs.clone(), rhs.clone());
            // Use l2_rhs for VC
            lhs = self.exec_state.new_symbol(&lhs, Level::Level2);
            if lhs.ty().is_zero_sized_type() || rhs.is_type() {
                return;
            }
            self.vc_system.borrow_mut().assign(lhs, rhs, self.exec_state.cur_span());
        }
    }

    pub fn unwind(&mut self, pc: Pc) {
        if self.top().function.is_loop_entry(pc) {
            if self.top().loop_stack.is_empty() || self.top().loop_stack.last().unwrap().0 != pc {
                // New loop
                self.top_mut().loop_stack.push((pc, 1));
            } else {
                self.top_mut().loop_stack.last_mut().unwrap().1 += 1;
            }
            println!(
                "Unwinding loop bb{pc} in {:?} for {} times",
                self.top().function.name(),
                self.top().loop_stack.last().unwrap().1
            );
        }
    }

    pub(super) fn memory_leak_check(&mut self) {
        for object in self.exec_state.objects.clone() {
            let mut l1_object = object.clone();
            self.exec_state.rename(&mut l1_object, Level::Level1);
            let object_state = self.exec_state.get_place_state(&l1_object);
            if object_state.is_dead() || object_state.is_own() {
                continue;
            }

            let msg = NString::from(format!("memory leak: {object:?} is not dealloced"));
            let ident = Ident::Global(NString::ALLOC_SYM);
            let alloac_array = self.exec_state.ns.lookup_object(ident);
            let address_of = self.ctx.address_of(object.clone(), object.extract_address_type());
            let base = self.ctx.pointer_base(address_of);
            let is_leak = self.ctx.index(alloac_array, base, Type::bool_type());
            self.claim(msg, is_leak.into());
        }
    }

    pub(super) fn make_project(&mut self, place: &Place) -> Expr {
        Projection::new(self).project(place)
    }

    pub(super) fn make_deref(&mut self, mut pt: Expr, mode: Mode, guard: Guard, ty: Type) -> Expr {
        self.replace_predicates(&mut pt);
        Projection::new(self).project_deref(pt, mode, guard, ty)
    }

    pub(super) fn make_mirconst(&mut self, mirconst: &MirConst) -> Expr {
        let ty = Type::from(mirconst.ty());
        match mirconst.kind() {
            ConstantKind::Ty(tyconst) => self.make_tyconst(tyconst),
            ConstantKind::Allocated(allocation) => {
                if allocation.provenance.ptrs.is_empty() {
                    self.make_allocation(allocation, ty)
                } else {
                    assert!(allocation.provenance.ptrs.len() == 1);
                    let (_, prov) = &allocation.provenance.ptrs[0];
                    self.make_global_alloc(prov, ty)
                }
            }
            ConstantKind::ZeroSized => self.ctx.constant_zst(ty),
            _ => panic!("Not support {:?}", mirconst.kind()),
        }
    }

    pub(super) fn make_tyconst(&mut self, tyconst: &TyConst) -> Expr {
        match tyconst.kind() {
            TyConstKind::Value(ty, _) => {
                let ty = Type::from(*ty);
                if ty.is_unsigned() {
                    let n = tyconst.eval_target_usize().expect("Not usize") as usize;
                    self.ctx.constant_integer(BigInt::from(n), ty)
                } else {
                    todo!()
                }
            }
            _ => todo!("{:?}", tyconst.kind()),
        }
    }

    pub(super) fn make_allocation(&mut self, allocation: &Allocation, ty: Type) -> Expr {
        if ty.is_ptr() {
            let is_null = allocation.is_null().expect("Must exists");
            assert!(is_null);
            return self.ctx.null(ty);
        }

        let value = self.make_allocation_rec(&allocation.bytes, ty);
        self.ctx.constant(value, ty)
    }

    fn make_allocation_rec(&mut self, allocation: &[Option<u8>], ty: Type) -> Constant {
        if ty.is_zero_sized_type() {
            Constant::Zst(ty)
        } else if ty.is_bool() {
            Constant::Bool(allocation[0].unwrap() != 0)
        } else if ty.is_integer() {
            let bytes = allocation
                .iter()
                .filter(|&&byte| byte != None)
                .map(|byte| byte.unwrap())
                .collect::<Vec<_>>();
            assert!(bytes.len() == ty.size());
            Constant::Integer(read_target_integer(&bytes))
        } else if ty.is_struct() || ty.is_tuple() {
            let shape = ty.shape();
            let n = shape.fields.count();
            let mut fields = Vec::new();
            if n == 1 {
                fields.push(self.make_allocation_rec(allocation, ty.field_type(0)));
            } else {
                let field_offsets = match &shape.fields {
                    FieldsShape::Arbitrary { offsets } => {
                        offsets.iter().map(|x| x.bytes()).collect::<Vec<_>>()
                    }
                    _ => panic!("Impossible"),
                };
                let is_increasing = field_offsets[1] > field_offsets[0];
                for i in 0..n {
                    let (l, r) = if is_increasing {
                        if i < n - 1 {
                            (field_offsets[i], field_offsets[i + 1])
                        } else {
                            (field_offsets[i], allocation.len())
                        }
                    } else {
                        if i > 0 {
                            (field_offsets[i], field_offsets[i - 1])
                        } else {
                            (field_offsets[i], allocation.len())
                        }
                    };
                    fields.push(self.make_allocation_rec(&allocation[l..r], ty.field_type(i)));
                }
            }
            Constant::Adt(fields, ty)
        } else if ty.is_enum() {
            let align = ty.align();
            let variant_idx_bytes =
                allocation[0..align].iter().map(|&byte| byte.unwrap()).collect::<Vec<_>>();
            let i = bigint_to_usize(&read_target_integer(&variant_idx_bytes));
            let idx = Constant::Integer(BigInt::from(i));
            let variant_type = ty.enum_variant_data_type(i);
            if variant_type.is_zero_sized_type() {
                Constant::Adt(vec![idx], ty)
            } else {
                let value = self.make_allocation_rec(&allocation[align..], variant_type);
                Constant::Adt(vec![idx, value], ty)
            }
        } else {
            todo!("Allocation {ty:?}")
        }
    }

    fn make_global_alloc(&mut self, prov: &Prov, ty: Type) -> Expr {
        let global_alloc = GlobalAlloc::from(prov.0);
        match global_alloc {
            GlobalAlloc::Static(def) => {
                // Since accessing global variables through pointers,
                // return its address.
                let ident = Ident::Global(def.trimmed_name().into());
                let object = self.exec_state.ns.lookup_object(ident);
                self.ctx.address_of(object.clone(), ty)
            }
            _ => panic!("Do not support global alloc {global_alloc:?}"),
        }
    }

    /// Return `l1` expr
    pub(super) fn make_operand(&mut self, operand: &Operand) -> Expr {
        match operand {
            Operand::Copy(p) => self.make_project(p),
            Operand::Move(p) => {
                let expr = self.make_project(p);
                self.ctx._move(expr)
            }
            Operand::Constant(op) => self.make_mirconst(&op.const_),
        }
    }

    /// Interface for `l2` reaming.
    pub(super) fn rename(&mut self, expr: &mut Expr) {
        self.exec_state.rename(expr, Level::Level2);
    }

    pub(super) fn replace_predicates(&self, expr: &mut Expr) {
        let mut sub_exprs = expr.sub_exprs();
        let mut has_changed = false;
        for sub_expr in sub_exprs.iter_mut() {
            if sub_expr.has_predicates() {
                has_changed |= true;
                self.replace_predicates(sub_expr);
            }
        }
        if has_changed {
            expr.replace_sub_exprs(sub_exprs);
        }

        if expr.is_valid() || expr.is_invalid() {
            let object = expr.extract_object();
            let base = self
                .ctx
                .pointer_base(self.ctx.address_of(object.clone(), object.extract_address_type()));
            let ident = Ident::Global(NString::ALLOC_SYM);
            let alloc_array = self.exec_state.ns.lookup_object(ident);
            let alloced = self.ctx.index(alloc_array, base, Type::bool_type());
            *expr = if expr.is_invalid() { self.ctx.not(alloced) } else { alloced };
            return;
        }

        if expr.is_move() {
            *expr = expr.extract_object();
            return;
        }
    }

    pub(super) fn assume(&self, mut cond: Expr) {
        if cond.is_true() {
            return;
        }
        cond.simplify();
        self.vc_system.borrow_mut().assume(cond);
    }

    /// Generating assertion in form: `path /\ error`,
    pub(super) fn claim(&mut self, msg: NString, mut error: Expr) {
        self.replace_predicates(&mut error);
        self.rename(&mut error);
        error.simplify();
        // The guard of current state is path condition.
        let mut guard = self.exec_state.cur_state.guard.clone();
        guard.add(error);
        if guard.is_false() {
            return;
        }
        let mut cond = guard.to_expr();
        cond.simplify();
        if cond.is_false() {
            return;
        }
        self.vc_system.borrow_mut().assert(msg, cond, self.exec_state.cur_span());
    }
}
