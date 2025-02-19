
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
use crate::solvers::smt::smt_array::Array;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_tuple::Tuple;
use crate::solvers::solver::Result;
use crate::NString;

pub struct Z3Conv<'ctx> {
  z3_ctx: &'ctx z3::Context,
  array_sorts: HashMap<Type, (z3::Sort<'ctx>, z3::Sort<'ctx>)>,
  tuple_sorts: HashMap<Type, z3::DatatypeSort<'ctx>>,
  z3_solver: z3::Solver<'ctx>,
}

impl<'ctx> Z3Conv<'ctx> {
  pub fn new(z3_ctx: &'ctx z3::Context) -> Self {
    let z3_solver = z3::Solver::new(z3_ctx);
    Z3Conv {
      z3_ctx,
      array_sorts: HashMap::default(),
      tuple_sorts: HashMap::default(),
      z3_solver
    }
  }
}

impl<'ctx> SmtSolver for Z3Conv<'ctx> {
  fn init(&mut self, program: &Program) {
    // self.set_arrays();
    self.set_tuples();
    self.set_arrays_from_program(program);
    self.set_tuples_from_program(program);
  }

  fn assert_assign(&self, lhs: Expr, rhs: Expr) {
    let a = self.convert_ast(lhs);
    let b = self.convert_ast(rhs);
    
    let res = a._eq(&b);

    println!("{res:?}");

    self.z3_solver.assert(&res);
  }

  fn assert_expr(&self, expr: Expr) {
    let a = self.convert_ast(expr);
    self.z3_solver.assert(&a.as_bool().expect("the assertion is not bool"));
  }

  fn push(&self) { self.z3_solver.push(); }
  
  fn pop(&self, n: u32) { self.z3_solver.pop(n); }
  
  fn reset(&self) { self.z3_solver.reset(); }

  fn dec_check(&self) -> Result {
    match self.z3_solver.check() {
      z3::SatResult::Unsat => Result::PUnsat,
      z3::SatResult::Unknown => Result::PUnknow,
      z3::SatResult::Sat => Result::PSat,
    }
  }
}

impl<'ctx> Convert<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn convert_pointer_sort(&self, ty: Type) -> z3::Sort<'ctx> {
    assert!(ty.is_any_ptr());
    let unit_ptr_type =
      Type::ptr_type(Type::unit_type(), Mutability::Not);
    self
      .tuple_sorts
      .get(&unit_ptr_type)
      .expect("Not pointer?")
      .sort
      .clone()
  }

  fn convert_array_sort(&self, ty: Type) -> z3::Sort<'ctx> {
    let (domain, range) =
      self.array_sorts.get(&ty).expect("Not array type");
    z3::Sort::array(&self.z3_ctx, domain, range)
  }

  fn convert_tuple_sort(&self, ty: Type) -> z3::Sort<'ctx> {
    self
      .tuple_sorts
      .get(&ty)
      .expect(format!("Tuple sort {ty:?} does not exists").as_str())
      .sort
      .clone()
  }


  fn convert_constant(&self, constant: &Constant, ty: Type) -> z3::ast::Dynamic<'ctx> {
    match constant {
      Constant::Bool(b) => self.mk_smt_bool(*b),
      Constant::Integer(s, i) => self.mk_smt_int(*s, *i),
      Constant::Struct(constants) => {
        let mut fields = Vec::new();
        for (c, st) in constants {
          fields.push(self.convert_constant(c, st.clone()));
        }
        let dtsort =
          self.tuple_sorts.get(&ty).expect("Something wrong");
        self.create_tuple(fields, dtsort)
      },
    }
  }

  fn convert_symbol(&self, name: NString, ty: Type) -> z3::ast::Dynamic<'ctx> {
    if ty.is_bool() { return self.mk_bool_symbol(name); }
    if ty.is_integer() { return self.mk_int_symbol(name); }
    if ty.is_any_ptr() {
      let sort = self.convert_pointer_sort(ty);
      return self.mk_tuple_symbol(name, sort);
    }
    if ty.is_array() {
      let (domain, range) =
        self
          .array_sorts
          .get(&ty)
          .expect(format!("Array type {ty:?} does not exists").as_str());
      return self.mk_array_symbol(name, domain.clone(), range.clone());
    }
    if ty.is_struct() {
      let sort = self.convert_tuple_sort(ty);
      return self.mk_tuple_symbol(name, sort)
    }
    panic!("{ty:?} symbol is not support?")
  }

  fn convert_address_of(&self, object: Expr) -> z3::ast::Dynamic<'ctx> {
    assert!(object.is_object());
    // add index_of later
    let symbol = object.extract_inner_object();
    assert!(symbol.is_symbol());
    let ty = object.ty();
    todo!()
  }

  fn mk_bool_sort(&self) -> z3::Sort<'ctx> {
    z3::Sort::bool(&self.z3_ctx)
  }

  fn mk_int_sort(&self) -> z3::Sort<'ctx> {
    z3::Sort::int(&self.z3_ctx)
  }

  fn mk_smt_bool(&self, b: bool) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Bool::from_bool(&self.z3_ctx, b))
  }

  fn mk_smt_int(&self, sign: Sign, i: u128) -> z3::ast::Dynamic<'ctx> {
    let num = if sign { "-" } else { "" }.to_string() + i.to_string().as_str();
    z3::ast::Dynamic::from(
      z3::ast::Int::from_str(&self.z3_ctx, &num)
      .expect("Wrong integer")
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
    domain: z3::Sort<'ctx>,
    range: z3::Sort<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Array::new_const(
        &self.z3_ctx, 
        name.to_string(), 
        &domain, 
        &range
      )
    )
  }

  fn mk_tuple_symbol(
    &self,
    name: NString,
    sort: z3::Sort<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    assert!(sort.kind() == z3::SortKind::Datatype);
    z3::ast::Dynamic::from(
      z3::ast::Datatype::new_const(&self.z3_ctx, name.to_string(), &sort)
    )
  }

  fn mk_add(
    &self,
    lhs: z3::ast::Dynamic<'ctx>,
    rhs: z3::ast::Dynamic<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") +
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_sub(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") -
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_mul(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") *
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_div(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer") /
      rhs.as_int().expect("rhs is not integer")
    )
  }

  fn mk_eq(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        ._eq(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_ne(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(z3::ast::Bool::not(&self.mk_eq(lhs, rhs).as_bool().unwrap()))
  }

  fn mk_ge(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .ge(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_gt(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .gt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_le(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .le(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_lt(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      lhs.as_int().expect("lhs is not integer")
        .lt(&rhs.as_int().expect("rhs is not integer"))
    )
  }

  fn mk_and(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Bool::and(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_or(&self, lhs: z3::ast::Dynamic<'ctx>, rhs: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      z3::ast::Bool::or(
        &self.z3_ctx, 
        &[&lhs.as_bool().expect("lhs is not bool"),
        &rhs.as_bool().expect("rhs is not bool")]
      )
    )
  }

  fn mk_not(&self, operand: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(operand.as_bool().expect("operand is no bool").not())
  }

  fn mk_implies(&self, cond: z3::ast::Dynamic<'ctx>, conseq: z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    z3::ast::Dynamic::from(
      cond
        .as_bool()
        .expect("cond is not bool")
        .implies(&conseq.as_bool().expect("conseq is not bool"))
    )
  }
}


impl<'ctx> Array<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn set_arrays(&mut self) {
      todo!()
  }

  fn set_arrays_from_program(&mut self, program: &Program) {
    let mut array_types = HashSet::new();
    for func in program.functions() {
      for local in func.locals() {
        let ty = Type::from(local.ty);
        if ty.is_array() { array_types.insert(ty); }
      }
    }
    for array_type in array_types {
      let domain = self.convert_sort(array_type.array_domain());
      let range = self.convert_sort(array_type.array_range());
      self.array_sorts.insert(array_type, (domain, range));
    }
  }
}

impl<'ctx> Tuple<z3::DatatypeSort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn set_tuples(&mut self) {
    // A pointer is a tuple (base, offset)
    let pointer_tuple_sort =
      z3::DatatypeBuilder
        ::new(&self.z3_ctx, "pointer")
        .variant(
          "pointer",
          vec![
            ("base", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx))),
            ("offset", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx)))
            ]
          )
        .finish();
    let void = Type::unit_type();
    println!("is unit ? {}", void.is_unit());
    // Use `*unit` as key for storage.
    let unit_ptr_type =
      Type::ptr_type(Type::unit_type(), Mutability::Not);
    self.tuple_sorts.insert(unit_ptr_type, pointer_tuple_sort);
  }

  fn set_tuples_from_program(&mut self, program: &Program) {
    let mut struct_types = HashSet::new();
    for func in program.functions() {
      for local in func.locals() {
        let ty = Type::from(local.ty);
        if ty.is_struct() { struct_types.insert(ty); }
      }
    }
    for struct_type in struct_types {
      let struct_def = self.create_tuple_sort(struct_type);
      self.tuple_sorts.insert(struct_type, struct_def);
    }
  }

  fn create_tuple_sort(&self, ty: Type) -> z3::DatatypeSort<'ctx> {
    assert!(ty.is_struct());
    let def = ty.struct_def();

    let name = "_struct_".to_string() + def.0.to_string().as_str();
    let mut fields = Vec::new();
    for field in def.1 {
      let accessor =
        z3::DatatypeAccessor::Sort(self.convert_sort(field.1));
      fields.push((field.0.as_str(), accessor));
    }

    z3::DatatypeBuilder
      ::new(&self.z3_ctx, name.clone())
      .variant(name.as_str(), fields)
      .finish()
  }
  
  fn create_tuple(
    &self,
    fields: Vec<z3::ast::Dynamic<'ctx>>,
    sort: &z3::DatatypeSort<'ctx>)
    -> z3::ast::Dynamic<'ctx> {
    assert!(sort.variants.len() == 1);
    let f = &sort.variants[0].constructor;
    let mut args = Vec::new();
    for arg in fields.iter() { 
      args.push(arg as &dyn Ast<'_>);
    }
    f.apply(args.as_slice())
  }
}