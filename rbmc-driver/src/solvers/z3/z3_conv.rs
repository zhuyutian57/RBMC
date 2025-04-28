use std::collections::HashMap;

use num_bigint::BigInt;

use z3;
use z3::ast::Ast;

use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::program::program::bigint_to_usize;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_datatype::*;
use crate::solvers::smt::smt_memspace::*;
use crate::solvers::solver::PResult;
use crate::symbol::nstring::NString;

pub struct Z3Conv<'ctx> {
    pub(super) z3_ctx: &'ctx z3::Context,
    z3_solver: z3::Solver<'ctx>,
    pub(super) fresh_count: HashMap<NString, usize>,
    pub(super) datatypes: HashMap<DataTypeSign, z3::DatatypeSort<'ctx>>,
    pub(super) pointer_logic: PointerLogic<z3::ast::Dynamic<'ctx>>,
    /// Cache Ast
    cache: HashMap<Expr, z3::ast::Dynamic<'ctx>>,
    /// Cache current alloc.
    pub(super) cur_alloc_expr: Option<z3::ast::Dynamic<'ctx>>,
}

impl<'ctx> Z3Conv<'ctx> {
    pub fn new(z3_ctx: &'ctx z3::Context) -> Self {
        let z3_solver = z3::Solver::new(z3_ctx);
        Z3Conv {
            z3_ctx,
            z3_solver,
            fresh_count: HashMap::new(),
            datatypes: HashMap::new(),
            pointer_logic: PointerLogic::new(),
            cache: HashMap::new(),
            cur_alloc_expr: None,
        }
    }

    pub(super) fn fresh_symbol(&mut self, prefix: NString) -> NString {
        self.fresh_count.entry(prefix).and_modify(|c| *c += 1).or_insert(1);
        prefix + "-" + self.fresh_count.get(&prefix).unwrap().to_string()
    }

    pub(super) fn assert(&self, e: z3::ast::Dynamic<'ctx>) {
        self.z3_solver.assert(&e.as_bool().unwrap());
    }
}

impl<'ctx> SmtSolver<'ctx> for Z3Conv<'ctx> {
    fn init(&mut self) {
        self.set_pointer_logic();
    }

    fn assert_assign(&mut self, lhs: Expr, rhs: Expr) {
        // For correct data-flow, a.k.a ALLOC_SYM, we must translate rhs firstly.
        // Because before changing ALLOC_SYM in lhs, the current one may be used
        // by address_of due to the constant propagation.
        let b = self.convert_ast(rhs.clone());
        let a = self.convert_ast(lhs.clone());

        let res = a._eq(&b);

        self.assert(z3::ast::Dynamic::from(res));

        self.cache_ast(lhs, b);
    }

    fn assert_expr(&mut self, expr: Expr) {
        let e = self.convert_ast(expr);
        self.assert(e);
    }

    fn reset(&mut self) {
        // Clear solver assertions
        self.z3_solver.reset();
        // Clear cache
        self.cache.clear();
        // Clear memory space
        self.pointer_logic.clear();
        // Reset alloc array
        self.cur_alloc_expr = None;
    }

    fn check(&self) -> PResult {
        match self.z3_solver.check() {
            z3::SatResult::Sat => PResult::PSat,
            z3::SatResult::Unknown => PResult::PUnknow,
            z3::SatResult::Unsat => PResult::PUnsat,
        }
    }

    fn eval_bool(&self, expr: Expr) -> bool {
        let ast = self.get_cache_ast(&expr).expect("Not put into solver");
        self.z3_solver
            .get_model()
            .expect("No model")
            .eval(&ast, true)
            .expect("Model does not interprete this expr")
            .as_bool()
            .unwrap()
            .as_bool()
            .expect("Wrong result")
    }

    fn show_model(&self) {
        match self.z3_solver.get_model() {
            Some(m) => println!("{m:?}"),
            None => println!("None"),
        };
    }
}

impl<'ctx> Convert<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
    fn cache_ast(&mut self, expr: Expr, ast: z3::ast::Dynamic<'ctx>) {
        self.cache.entry(expr).and_modify(|x| *x = ast.clone()).or_insert(ast);
    }

    fn get_cache_ast(&self, expr: &Expr) -> Option<z3::ast::Dynamic<'ctx>> {
        self.cache.get(expr).cloned()
    }

    fn cache_alloc_ast(&mut self, ast: z3::ast::Dynamic<'ctx>) {
        self.cur_alloc_expr = Some(ast);
    }

    fn convert_struct_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
        self.mk_struct_sort(ty)
    }

    fn convert_tuple_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
        self.mk_tuple_sort(ty)
    }

    fn convert_enum_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
        self.mk_enum_sort(ty)
    }

    fn convert_null(&self, ty: Type) -> z3::ast::Dynamic<'ctx> {
        let null_pt =
            self.mk_pointer(&self.mk_smt_int(BigInt::ZERO), &self.mk_smt_int(BigInt::ZERO), None);
        if ty.is_primitive_ptr() {
            null_pt
        } else if ty.is_box() {
            self.mk_box(&null_pt)
        } else {
            panic!("Not support null({ty:?})")
        }
    }

    fn convert_pointer(
        &self,
        base: &z3::ast::Dynamic<'ctx>,
        offset: &z3::ast::Dynamic<'ctx>,
        meta: Option<&z3::ast::Dynamic<'ctx>>,
    ) -> z3::ast::Dynamic<'ctx> {
        self.mk_pointer(base, offset, meta)
    }

    fn convert_pointer_base(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        if pt.get_sort() == self.mk_int_sort() { return pt.clone(); }
        self.mk_pointer_base(pt)
    }

    fn convert_pointer_offset(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        self.mk_pointer_offset(pt)
    }

    fn convert_pointer_meta(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        self.mk_pointer_meta(pt)
    }

    fn convert_box(&self, _box: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        self.mk_box(_box)
    }

    fn convert_vec(
        &self,
        _vec: &z3::ast::Dynamic<'ctx>,
        len: &z3::ast::Dynamic<'ctx>,
        cap: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        self.mk_vec(_vec, len, cap)
    }

    fn convert_vec_len(&self, _vec: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        self.mk_vec_len(_vec)
    }

    fn convert_vec_cap(&self, _vec: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        self.mk_vec_cap(_vec)
    }

    fn convert_inner_pointer(
        &self,
        pt: &z3::ast::Dynamic<'ctx>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        assert!(ty.is_box() || ty.is_vec());
        if ty.is_box() { self.mk_box_ptr(pt) } else { self.mk_vec_ptr(pt) }
    }

    fn convert_struct(
        &mut self,
        fields: &Vec<z3::ast::Dynamic<'ctx>>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        self.mk_struct(fields, ty)
    }

    fn convert_tuple(
        &mut self,
        fields: &Vec<z3::ast::Dynamic<'ctx>>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        self.mk_tuple(fields, ty)
    }

    fn convert_enum(
        &mut self,
        idx: usize,
        data: Option<z3::ast::Dynamic<'ctx>>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        self.mk_variant(idx, data, ty)
    }

    fn convert_object_space(&mut self, object: &Expr) -> z3::ast::Dynamic<'ctx> {
        assert!(object.is_object());
        let mut inner_expr = object.extract_inner_expr();
        if inner_expr.is_as_variant() {
            inner_expr = inner_expr.extract_enum();
        }
        self.create_object_space(&inner_expr)
    }

    /// Select from struct/tuple
    fn convert_index_tuple(&mut self, object: Expr, field: Expr) -> z3::ast::Dynamic<'ctx> {
        let i = bigint_to_usize(&field.extract_integer());
        let ty = object.ty();
        let object = self.convert_ast(object);
        self.mk_tuple_select(object, i, ty)
    }

    fn convert_index_enum(&mut self, object: Expr, field: Expr) -> z3::ast::Dynamic<'ctx> {
        let ty = object.ty();
        let sign = self.create_datatype_sign(ty);
        let as_variant = object.extract_inner_expr();
        assert!(as_variant.is_as_variant());
        let idx = as_variant.extract_variant_idx();
        let i = bigint_to_usize(&field.extract_integer());
        let args = &[&self.convert_ast(object) as &dyn Ast];
        self.datatypes.get(&sign).unwrap().variants[idx].accessors[i].apply(args)
    }

    fn convert_tuple_update(
        &mut self,
        object: Expr,
        field: Expr,
        value: Expr,
    ) -> z3::ast::Dynamic<'ctx> {
        let i = bigint_to_usize(&field.extract_constant().to_integer());
        let ty = object.ty();
        let object = self.convert_ast(object);
        let value = self.convert_ast(value);
        self.mk_tuple_store(object, i, value, ty)
    }

    fn convert_variant_update(
        &mut self,
        variant: Expr,
        field: Expr,
        value: Expr,
    ) -> z3::ast::Dynamic<'ctx> {
        assert!(variant.is_as_variant());
        let _enum = variant.extract_enum();
        let variant_idx = variant.extract_variant_idx();
        let variant_data_type = _enum.ty().enum_variant_data_type(variant_idx);

        let object = self.convert_ast(variant);
        let value = self.convert_ast(value);
        let i = bigint_to_usize(&field.extract_constant().to_integer());
        let data = self.mk_tuple_store(object, i, value, variant_data_type);
        // Create a variant
        self.mk_variant(variant_idx, Some(data), _enum.ty())
    }

    fn convert_as_variant(&mut self, _enum: Expr, idx: usize) -> z3::ast::Dynamic<'ctx> {
        self.mk_as_variant(_enum, idx)
    }

    fn convert_match_variant(&mut self, _enum: Expr, idx: usize) -> z3::ast::Dynamic<'ctx> {
        self.mk_match_variant(_enum, idx)
    }

    fn mk_fresh(&mut self, prefix: NString, ty: Type) -> z3::ast::Dynamic<'ctx> {
        let fresh_symbol = self.fresh_symbol(prefix);
        self.convert_symbol(fresh_symbol, ty)
    }

    fn mk_bool_sort(&self) -> z3::Sort<'ctx> {
        z3::Sort::bool(&self.z3_ctx)
    }

    fn mk_int_sort(&self) -> z3::Sort<'ctx> {
        z3::Sort::int(&self.z3_ctx)
    }

    fn mk_array_sort(&mut self, domain: &z3::Sort<'ctx>, range: &z3::Sort<'ctx>) -> z3::Sort<'ctx> {
        z3::Sort::array(&self.z3_ctx, domain, range)
    }

    fn mk_pointer_sort(&self) -> z3::Sort<'ctx> {
        self.pointer_sort()
    }

    fn mk_vec_sort(&self) -> z3::Sort<'ctx> {
        self.vec_sort()
    }

    fn mk_smt_bool(&self, b: bool) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Bool::from_bool(&self.z3_ctx, b))
    }

    fn mk_smt_int(&self, i: BigInt) -> z3::ast::Dynamic<'ctx> {
        let num = i.to_string();
        z3::ast::Dynamic::from(
            z3::ast::Int::from_str(&self.z3_ctx, num.as_str()).expect("Wrong integer"),
        )
    }

    fn mk_smt_const_array(
        &self,
        domain: &z3::Sort<'ctx>,
        val: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Array::const_array(&self.z3_ctx, domain, val))
    }

    fn mk_bool_symbol(&self, name: NString) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Bool::new_const(&self.z3_ctx, name.to_string()))
    }

    fn mk_int_symbol(&self, name: NString) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Int::new_const(&self.z3_ctx, name.to_string()))
    }

    fn mk_array_symbol(
        &self,
        name: NString,
        domain: &z3::Sort<'ctx>,
        range: &z3::Sort<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Array::new_const(
            &self.z3_ctx,
            name.to_string(),
            domain,
            range,
        ))
    }

    fn mk_tuple_symbol(&self, name: NString, sort: &z3::Sort<'ctx>) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Datatype::new_const(&self.z3_ctx, name.to_string(), sort))
    }

    fn mk_enum_symbol(&self, name: NString, sort: &z3::Sort<'ctx>) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Datatype::new_const(&self.z3_ctx, name.to_string(), sort))
    }

    fn project(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        self.mk_pointer_base(&pt)
    }

    fn mk_select(
        &self,
        array: &z3::ast::Dynamic<'ctx>,
        index: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        array.as_array().unwrap().select(index)
    }

    fn mk_store(
        &self,
        array: &z3::ast::Dynamic<'ctx>,
        index: &z3::ast::Dynamic<'ctx>,
        val: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(array.as_array().unwrap().store(index, val))
    }

    fn mk_add(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int().expect("lhs is not integer") + rhs.as_int().expect("rhs is not integer"),
        )
    }

    fn mk_sub(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int().expect("lhs is not integer") - rhs.as_int().expect("rhs is not integer"),
        )
    }

    fn mk_mul(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int().expect("lhs is not integer") * rhs.as_int().expect("rhs is not integer"),
        )
    }

    fn mk_div(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int().expect("lhs is not integer") / rhs.as_int().expect("rhs is not integer"),
        )
    }

    fn mk_eq(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(lhs._eq(rhs))
    }

    fn mk_ne(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(&self.mk_eq(lhs, rhs).as_bool().unwrap().not())
    }

    fn mk_ge(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int()
                .expect("lhs is not integer")
                .ge(&rhs.as_int().expect("rhs is not integer")),
        )
    }

    fn mk_gt(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int()
                .expect("lhs is not integer")
                .gt(&rhs.as_int().expect("rhs is not integer")),
        )
    }

    fn mk_le(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int()
                .expect("lhs is not integer")
                .le(&rhs.as_int().expect("rhs is not integer")),
        )
    }

    fn mk_lt(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            lhs.as_int()
                .expect("lhs is not integer")
                .lt(&rhs.as_int().expect("rhs is not integer")),
        )
    }

    fn mk_and(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Bool::and(
            &self.z3_ctx,
            &[&lhs.as_bool().expect("lhs is not bool"), &rhs.as_bool().expect("rhs is not bool")],
        ))
    }

    fn mk_or(
        &self,
        lhs: &z3::ast::Dynamic<'ctx>,
        rhs: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(z3::ast::Bool::or(
            &self.z3_ctx,
            &[&lhs.as_bool().expect("lhs is not bool"), &rhs.as_bool().expect("rhs is not bool")],
        ))
    }

    fn mk_not(&self, operand: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(operand.as_bool().expect("operand is no bool").not())
    }

    fn mk_implies(
        &self,
        cond: &z3::ast::Dynamic<'ctx>,
        conseq: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        z3::ast::Dynamic::from(
            cond.as_bool()
                .expect("cond is not bool")
                .implies(&conseq.as_bool().expect("conseq is not bool")),
        )
    }

    fn mk_ite(
        &self,
        cond: &z3::ast::Dynamic<'ctx>,
        true_value: &z3::ast::Dynamic<'ctx>,
        false_value: &z3::ast::Dynamic<'ctx>,
    ) -> z3::ast::Dynamic<'ctx> {
        cond.as_bool().expect("condition must be bool").ite(true_value, false_value)
    }
}
