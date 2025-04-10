use num_bigint::BigInt;
use z3::DatatypeAccessor;
use z3::ast::*;

use super::z3_conv::*;
use crate::expr::expr::*;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_memspace::*;
use crate::symbol::nstring::NString;

impl<'ctx> MemSpace<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
    fn set_pointer_logic(&mut self) {
        // A pointer is a tuple (base, offset)
        let pointer_tuple_sort = z3::DatatypeBuilder::new(&self.z3_ctx, "pointer")
            .variant(
                "pointer",
                vec![
                    ("base", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx))),
                    ("offset", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx))),
                    ("meta", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx))),
                ],
            )
            .finish();
        self.datatypes.insert((NString::from("pointer"), vec![]), pointer_tuple_sort);

        // General pointer sort
        let pointer_sort = self.pointer_sort();

        // A box pointer is a tuple (pointer)
        let box_tuple_sort = z3::DatatypeBuilder::new(&self.z3_ctx, "box")
            .variant("box", vec![("box_ptr", DatatypeAccessor::Sort(pointer_sort.clone()))])
            .finish();
        self.datatypes.insert((NString::from("box"), vec![]), box_tuple_sort);

        // A vec pointer is a tuple (pointer, len, cap)
        let vec_tuple_sort = z3::DatatypeBuilder::new(&self.z3_ctx, "vec")
            .variant(
                "vec",
                vec![
                    ("vec_ptr", DatatypeAccessor::Sort(pointer_sort.clone())),
                    ("vec_len", DatatypeAccessor::Sort(self.mk_int_sort())),
                    ("vec_cap", DatatypeAccessor::Sort(self.mk_int_sort()))
                ])
            .finish();
    self.datatypes.insert((NString::from("vec"), vec![]), vec_tuple_sort);

    }

    fn pointer_sort(&self) -> z3::Sort<'ctx> {
        self.datatypes
            .get(&(NString::from("pointer"), vec![]))
            .expect("Pointer tuple is not initialized")
            .sort
            .clone()
    }

    fn box_sort(&self) -> z3::Sort<'ctx> {
        self.datatypes
            .get(&(NString::from("box"), vec![]))
            .expect("Box pointer tuple is not initialized")
            .sort
            .clone()
    }

    fn create_object_space(&mut self, object: &Expr) -> z3::ast::Dynamic<'ctx> {
        assert!(object.is_symbol());
        if self.pointer_logic.contains(object) {
            return self.pointer_logic.get_object_space_base(object);
        }
        self.init_pointer_space(object);
        self.pointer_logic.get_object_space_base(object)
    }

    fn init_pointer_space(&mut self, object: &Expr) {
        assert!(!self.pointer_logic.contains(object));
        assert!(object.is_symbol());
        let ty = object.ty();

        // Use l0 as identifier
        let space_base = NString::from(object.extract_symbol().ident()) + "_base";
        let base = self.mk_int_symbol(space_base);
        let len = if ty.is_array() && ty.array_size() == None {
            let sym = object.extract_symbol().ident() + "_size";
            self.mk_int_symbol(sym)
        } else {
            self.mk_smt_int(BigInt::from(ty.num_fields()))
        };

        // Base is greater than 0
        self.assert(self.mk_gt(&base, &self.mk_smt_int(BigInt::ZERO)));
        // Size is greater or eqaul to 0
        self.assert(self.mk_ge(&len, &self.mk_smt_int(BigInt::ZERO)));
        // Disjoint relationship
        // TODO: remove own object?
        for (b, l) in self.pointer_logic.object_spaces().values() {
            if space_base == NString::from(b.to_string()) {
                continue;
            }
            // No alloc array is active. That means we know the allocation of current
            // object in symex. No need to encode disjointness.
            // Be careful for this design.
            if self.cur_alloc_expr == None {
                continue;
            }

            let alloc_array_ast = self.cur_alloc_expr.as_ref().unwrap();
            let alive = alloc_array_ast.as_array().unwrap().select(b);

            let l1 = base.clone();
            let r1 = self.mk_add(&l1, &len);
            let l2 = b.clone();
            let r2 = self.mk_add(&l2, &l);
            let no_overlap = self.mk_or(&self.mk_le(&r1, &l2), &self.mk_le(&r2, &l1));
            let disj = self.mk_implies(&alive, &no_overlap);
            self.assert(disj);
        }

        self.pointer_logic.set_object_space(object.clone(), (base, len));
    }

    fn mk_pointer(
        &self,
        base: &z3::ast::Dynamic<'ctx>,
        offset: &z3::ast::Dynamic<'ctx>,
        meta: Option<&z3::ast::Dynamic<'ctx>>,
    ) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("pointer"), vec![]);
        let metadata = match meta {
            Some(x) => x.clone(),
            None => self.mk_smt_int(0.into()),
        };
        self.datatypes.get(&sign).unwrap().variants[0].constructor.apply(&[
            base as &dyn Ast,
            offset as &dyn Ast,
            &metadata as &dyn Ast,
        ])
    }

    fn mk_pointer_base(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("pointer"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[0]
            .apply(&[pt as &dyn Ast])
    }

    fn mk_pointer_offset(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("pointer"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[1]
            .apply(&[pt as &dyn Ast])
    }

    fn mk_pointer_meta(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("pointer"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[2]
            .apply(&[pt as &dyn Ast])
    }

    fn mk_box(&self, inner_pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("box"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].constructor
            .apply(&[inner_pt as &dyn Ast])
    }

    fn mk_box_ptr(&self, _box: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("box"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[0]
            .apply(&[_box as &dyn Ast])
    }

    fn mk_vec(
        &self,
        inner_pt: &z3::ast::Dynamic<'ctx>,
        len: &z3::ast::Dynamic<'ctx>,
        cap: &z3::ast::Dynamic<'ctx>
    ) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("vec"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].constructor
            .apply(&[inner_pt as &dyn Ast])
    }

    fn mk_vec_ptr(&self, _vec: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("vec"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[0]
            .apply(&[_vec as &dyn Ast])
    }

    fn mk_vec_len(&self, _vec: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("vec"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[1]
            .apply(&[_vec as &dyn Ast])
    }

    fn mk_vec_cap(&self, _vec: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("vec"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[2]
            .apply(&[_vec as &dyn Ast])
    }
}
