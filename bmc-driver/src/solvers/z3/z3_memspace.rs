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
    }

    fn pointer_sort(&self) -> z3::Sort<'ctx> {
        self.datatypes
            .get(&(NString::from("pointer"), vec![]))
            .expect("Pointer tuple is not initialized")
            .sort
            .clone()
    }

    fn create_object_space(&mut self, object: &Expr) -> z3::ast::Dynamic<'ctx> {
        assert!(object.is_symbol());
        if !self.pointer_logic.contains(object) {
            self.init_pointer_space(object);
        }
        let ident = self.pointer_logic.get_object_space_ident(object);
        self.mk_smt_int(BigInt::from(ident))
    }

    fn init_pointer_space(&mut self, object: &Expr) {
        let i = self.pointer_logic.add_object(object.clone());
        let ty = object.ty();
        let symbol = object.extract_symbol();

        // Use l0 as identifier. Object size is in byte-level
        let space_start = NString::from(object.extract_symbol().ident()) + "_base";
        let start = self.mk_int_symbol(space_start);
        let size = self.mk_smt_int(BigInt::from(ty.size()));
        let end = self.mk_add(&start, &size);
        // Start is greater than 0
        self.assert(self.mk_gt(&start, &self.mk_smt_int(BigInt::ZERO)));
        // Size is greater or eqaul to 0
        self.assert(self.mk_ge(&size, &self.mk_smt_int(BigInt::ZERO)));
        // Disjoint relationship
        for (j, (s, e)) in self.pointer_logic.object_spaces().values().enumerate() {
            if j == i {
                continue;
            }

            // No alloc array is active. That means we know the allocation of current
            // object in symex. No need to encode disjointness.
            if self.cur_alloc_expr == None {
                continue;
            }

            let alloc_array_ast = self.cur_alloc_expr.as_ref().unwrap();
            let ident = self.mk_smt_int(BigInt::from(j));
            let alive = alloc_array_ast.as_array().unwrap().select(&ident);

            let no_overlap = self.mk_or(&self.mk_le(&end, &s), &self.mk_le(&e, &start));
            self.assert(self.mk_implies(&alive, &no_overlap));
        }

        self.pointer_logic.set_object_space(object.clone(), (start, end));
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
        self.datatypes.get(&sign).unwrap().variants[0].accessors[0].apply(&[pt as &dyn Ast])
    }

    fn mk_pointer_offset(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("pointer"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[1].apply(&[pt as &dyn Ast])
    }

    fn mk_pointer_meta(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        let sign = (NString::from("pointer"), vec![]);
        self.datatypes.get(&sign).unwrap().variants[0].accessors[2].apply(&[pt as &dyn Ast])
    }
}
