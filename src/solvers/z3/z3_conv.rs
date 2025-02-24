
use std::collections::{HashMap, HashSet};

use stable_mir::mir::Mutability;

use z3;
use z3::DatatypeAccessor;
use z3::SortKind;
use z3::ast::Ast;

use crate::expr::constant::*;
use crate::expr::expr::*;
use crate::expr::ty::Type;
use crate::program::program::Program;
use crate::solvers::smt::smt_array::*;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_memspace::*;
use crate::solvers::smt::smt_tuple::*;
use crate::solvers::solver::PResult;
use crate::NString;

pub struct Z3Conv<'ctx> {
  z3_ctx: &'ctx z3::Context,
  z3_solver: z3::Solver<'ctx>,
  tuple_sorts: HashMap<Type, z3::DatatypeSort<'ctx>>,
  /// Use for making pointer tuple
  pointer_type: Type,
  pointer_logic: PointerLogic<z3::ast::Dynamic<'ctx>>,
  /// Cache Ast
  cache: HashMap<Expr, z3::ast::Dynamic<'ctx>>,
}

impl<'ctx> Z3Conv<'ctx> {
  pub fn new(z3_ctx: &'ctx z3::Context) -> Self {
    let z3_solver = z3::Solver::new(z3_ctx);
    Z3Conv {
      z3_ctx,
      z3_solver,
      tuple_sorts: HashMap::new(),
      pointer_type: Type::ptr_type(Type::unit_type(), Mutability::Not),
      pointer_logic: PointerLogic::new(),
      cache: HashMap::new(),
    }
  }

  fn assert(&self, e: z3::ast::Dynamic<'ctx>) {
    println!("{e:?}");
    self.z3_solver.assert(&e.as_bool().expect("the assertion is not bool"));
  }
}

impl<'ctx> SmtSolver for Z3Conv<'ctx> {
  fn init(&mut self, program: &Program) {
    self.set_pointer_logic();
  }

  fn assert_assign(&mut self, lhs: Expr, rhs: Expr) {
    let a = self.convert_ast(lhs);
    let b = self.convert_ast(rhs);

    let res = a._eq(&b);

    self.assert(z3::ast::Dynamic::from(res));
  }

  fn assert_expr(&mut self, expr: Expr) {
    let e = self.convert_ast(expr);
    self.assert(e);
  }
  
  fn reset(&self) { self.z3_solver.reset(); }

  fn check(&self) -> PResult {
    match self.z3_solver.check() {
      z3::SatResult::Unsat => PResult::PUnsat,
      z3::SatResult::Unknown => PResult::PUnknow,
      z3::SatResult::Sat => {
        println!("{:?}", self.z3_solver.get_model());
        PResult::PSat
      },
    }
  }
}

impl<'ctx> Convert<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn cache_ast(&mut self, expr: Expr, ast: z3::ast::Dynamic<'ctx>) {
    self
      .cache
      .entry(expr.clone())
      .and_modify(
        |x|
        panic!("Exists: {expr:?} = {x:?}")
      )
      .or_insert(ast);
  }

  fn get_cache_ast(&self, expr: &Expr) -> Option<z3::ast::Dynamic<'ctx>> {
    self.cache.get(expr).cloned()
  }

  fn convert_tuple_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
    self.create_tuple_sort(ty)
  }

  fn convert_pointer(
    &self,
    ident: &z3::ast::Dynamic<'ctx>,
    offset: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    self.mk_pointer(ident, offset)
  }

  fn convert_pointer_ident(
    &self,
    pt: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    self.mk_pointer_ident(pt)
  }

  fn convert_pointer_offset(
    &self,
    pt: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    self.mk_pointer_offset(pt)
  }

  fn convert_tuple(
    &mut self,
    fields: Vec<z3::ast::Dynamic<'ctx>>,
    ty: Type)
    -> z3::ast::Dynamic<'ctx> {
    self.create_tuple(fields, ty)
  }

  fn convert_object_space(&mut self, object: &Expr) -> z3::ast::Dynamic<'ctx> {
    self.create_object_space(object)
  }

  fn convert_tuple_load(
    &mut self,
    object: Expr,
    field: Expr)
    -> z3::ast::Dynamic<'ctx> {
    let i = field.extract_integer().to_uint() as usize;
    self.mk_tuple_select(object, i)
  }

  fn convert_tuple_update(
    &mut self,
    object: Expr,
    field: Expr,
    value: Expr)
    -> z3::ast::Dynamic<'ctx> {
    let i = field.extract_constant().to_integer().to_uint() as usize;
    self.mk_tuple_store(object, i, value)
  }

  fn mk_bool_sort(&self) -> z3::Sort<'ctx> {
    z3::Sort::bool(&self.z3_ctx)
  }

  fn mk_int_sort(&self) -> z3::Sort<'ctx> {
    z3::Sort::int(&self.z3_ctx)
  }

  fn mk_pointer_sort(&self) -> z3::Sort<'ctx> { self.pointer_sort() }

  fn mk_array_sort(
    &mut self,
    domain: &z3::Sort<'ctx>,
    range: &z3::Sort<'ctx>)
    -> z3::Sort<'ctx> {
    z3::Sort::array(&self.z3_ctx, domain, range)
  }

  fn mk_smt_bool(&self, b: bool) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Bool::from_bool(&self.z3_ctx, b))
  }

  fn mk_smt_int(&self, i: BigInt) -> z3::ast::Dynamic<'ctx> {
    let num = i.to_string();
    z3::ast::Dynamic::from(
      z3::ast::Int::from_str(&self.z3_ctx, num.as_str())
      .expect("Wrong integer")
    )
  }

  fn mk_smt_const_array(
    &self,
    domain: &z3::Sort<'ctx>,
    val: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Array::const_array(&self.z3_ctx, domain, val)
    )
  }

  fn mk_bool_symbol(&self, name: NString) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Bool::new_const(&self.z3_ctx, name.to_string())
    )
  }

  fn mk_int_symbol(&self, name: NString) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Int::new_const(&self.z3_ctx, name.to_string())
    )
  }

  fn mk_array_symbol(
    &self,
    name: NString,
    domain: &z3::Sort<'ctx>,
    range: &z3::Sort<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Array::new_const(
        &self.z3_ctx, 
        name.to_string(), 
        domain, range
      )
    )
  }

  fn mk_tuple_symbol(
    &self,
    name: NString,
    sort: &z3::Sort<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Datatype::new_const(&self.z3_ctx, name.to_string(), sort)
    )
  }

  fn project(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    self.mk_pointer_ident(pt)
  }

  fn mk_select(
    &self,
    array: &z3::ast::Dynamic<'ctx>,
    index: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    array.as_array().unwrap().select(index)
  }

  fn mk_store(
    &self,
    array: &z3::ast::Dynamic<'ctx>,
    index: &z3::ast::Dynamic<'ctx>,
    val: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(array.as_array().unwrap().store(index, val))
  }

  fn mk_add(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") +
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_sub(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") -
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_mul(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") *
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_div(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") /
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_eq(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        ._eq(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_ne(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Bool::not(&self.mk_eq(lhs, rhs).as_bool().unwrap()))
  }

  fn mk_ge(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .ge(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_gt(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .gt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_le(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .le(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_lt(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .lt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_and(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Bool::and(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_or(
    &self,
    lhs: &z3::ast::Dynamic<'ctx>,
    rhs: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Bool::or(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_not(
    &self,
    operand: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(operand.as_bool().expect("operand is no bool").not())
  }

  fn mk_implies(
    &self,
    cond: &z3::ast::Dynamic<'ctx>,
    conseq: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      cond
        .as_bool()
        .expect("cond is not bool")
        .implies(&conseq.as_bool().expect("conseq is not bool"))
    )
  }

  fn mk_ite(
    &self,
    cond: &z3::ast::Dynamic<'ctx>,
    true_value: &z3::ast::Dynamic<'ctx>,
    false_value: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    cond
      .as_bool()
      .expect("condition must be bool")
      .ite(true_value, false_value)
  }
}


impl<'ctx> Array<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {}

impl<'ctx> Tuple<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn create_tuple_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
    assert!(ty.is_struct());
    if self.tuple_sorts.contains_key(&ty) {
      return self.tuple_sorts.get(&ty).unwrap().sort.clone();
    }

    let def = ty.struct_def();
    let name = "_struct_".to_string() + def.0.to_string().as_str();
    let mut fields = Vec::new();
    for field in def.1 {
      let accessor =
        z3::DatatypeAccessor::Sort(self.convert_sort(field.1));
      fields.push((field.0.as_str(), accessor));
    }

    let dtsort =
      z3::DatatypeBuilder
        ::new(&self.z3_ctx, name.clone())
        .variant(name.as_str(), fields)
        .finish();
    let sort = dtsort.sort.clone();
    self.tuple_sorts.insert(ty, dtsort);
    sort
  }
  
  fn create_tuple(
    &mut self,
    fields: Vec<z3::ast::Dynamic<'ctx>>,
    ty: Type)
    -> z3::ast::Dynamic<'ctx> {
    if !self.tuple_sorts.contains_key(&ty) { self.create_tuple_sort(ty); }
    let dtsort = self.tuple_sorts.get(&ty).unwrap();
    let f = &dtsort.variants[0].constructor;
    let mut args = Vec::new();
    for arg in fields.iter() { 
      args.push(arg as &dyn Ast<'_>);
    }
    f.apply(args.as_slice())
  }

  fn mk_tuple_select(&mut self, object: Expr, field: usize) -> z3::ast::Dynamic<'ctx> {
    let args = 
      &[&self.convert_ast(object.clone()) as &dyn Ast];
    let dtsort =
      self.tuple_sorts.get(&object.ty())
      .expect(format!("{object:?} is not struct").as_str());
    assert!(field < dtsort.variants[0].accessors.len());
    dtsort
      .variants[0]
      .accessors[field]
      .apply(args)
  }
  
  fn mk_tuple_store(
    &mut self,
    object: Expr,
    field: usize,
    value: Expr)
    -> z3::ast::Dynamic<'ctx> {
    let n = self.tuple_sorts.get(&object.ty()).unwrap().variants[0].accessors.len();
    let mut fields_values = Vec::with_capacity(n);
    let update_value = self.convert_ast(value);
    for i in 0..n {
      if i != field {
        fields_values.push(self.mk_tuple_select(object.clone(), i));
      } else {
        fields_values.push(update_value.clone());
      }
    }
    let args =
      fields_values
        .iter()
        .map(|x| x as &dyn Ast)
        .collect::<Vec<_>>();
    self
      .tuple_sorts
      .get(&object.ty())
      .unwrap()
      .variants[0]
      .constructor
      .apply(&args.as_slice())
  }
}

impl<'ctx> MemSpace<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn set_pointer_logic(&mut self) {
    // A pointer is a tuple (base, offset)
    let pointer_tuple_sort =
      z3::DatatypeBuilder
        ::new(&self.z3_ctx, "pointer")
        .variant(
          "pointer",
          vec![
            ("ident", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx))),
            ("offset", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx)))
            ]
          )
        .finish();
    self.tuple_sorts.insert(self.pointer_type, pointer_tuple_sort);
  }

  fn pointer_sort(&self) -> z3::Sort<'ctx> {
    self
      .tuple_sorts
      .get(&self.pointer_type)
      .expect("Pointer tuple is not initialized")
      .sort
      .clone()
  }

  fn create_object_space(&mut self, object: &Expr) -> z3::ast::Dynamic<'ctx> {
    assert!(object.is_symbol());
    if self.pointer_logic.contains(object) {
      return self.pointer_logic.get_object_space_ident(object);
    }
    self.init_pointer_space(object);
    self.pointer_logic.get_object_space_ident(object)
  }

  fn init_pointer_space(&mut self, object: &Expr) {
    assert!(!self.pointer_logic.contains(object));
    assert!(object.is_symbol());

    // Use l0 as identifier
    let space_ident = NString::from(object.extract_symbol().ident());
    let space_base = space_ident + "_base";
    // The size is field-level
    let space_len =
      if object.ty().is_struct() {
        object.ty().struct_def().1.len()
      } else { 1 };

    let ident = self.mk_int_symbol(space_ident);
    let base = self.mk_int_symbol(space_base);
    let len = self.mk_smt_int(BigInt(false, space_len as u128));

    // Ident is greater than 0
    self.assert(self.mk_gt(&ident, &self.mk_smt_int(BigInt(false, 0))));
    // base is also greater than 0
    self.assert(self.mk_gt(&base, &self.mk_smt_int(BigInt(false, 0))));

    // TODO: set disjoint relationship
    
    self.pointer_logic.set_object_space(object.clone(), (ident, (base, len)));
  }

  fn mk_pointer(
    &self,
    base: &z3::ast::Dynamic<'ctx>,
    offset: &z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&self.pointer_type)
      .unwrap()
      .variants[0]
      .constructor
      .apply(&[base as &dyn Ast, offset as &dyn Ast])
  }

  fn mk_pointer_ident(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&self.pointer_type)
      .unwrap()
      .variants[0]
      .accessors[0]
      .apply(&[pt as &dyn Ast])
  }

  fn mk_pointer_offset(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&self.pointer_type)
      .unwrap()
      .variants[0]
      .accessors[1]
      .apply(&[pt as &dyn Ast])
  }
}