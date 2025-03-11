
use num_bigint::BigInt;
use z3::ast::*;
use z3::DatatypeAccessor;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::solvers::smt::smt_tuple::Variants;
use crate::NString;
use crate::program::program::*;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_tuple::*;
use super::z3_conv::*;

impl<'ctx> Tuple<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn create_tuple_sort(
    &mut self,
    name: NString,
    variants: Variants
  ) -> z3::Sort<'ctx> {
    let mut builder =
      z3::DatatypeBuilder::new(&self.z3_ctx, name.to_string());
    for variant in &variants {
      let mut fields = Vec::new();
      for (name, ty) in variant.1.iter() {
        let accessor =
          DatatypeAccessor::Sort(self.convert_sort(*ty));
        fields.push((name.as_str(), accessor));
      }
      builder = builder.variant(variant.0.as_str(), fields);
    }
    let dtsort = builder.finish();
    let sort = dtsort.sort.clone();
    self.tuple_sorts.insert(name, dtsort);
    sort
  }

  fn mk_struct_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
    assert!(ty.is_struct());
    let def = ty.struct_def();
    let tuple_name = NString::from("_struct_") + def.0;
    
    if self.tuple_sorts.contains_key(&tuple_name) {
      return self.tuple_sorts.get(&tuple_name).unwrap().sort.clone();
    }

    let mut fields = Vec::new();
    for (field, ty) in def.1.iter() {
      let field_name = tuple_name + "_" + *field;
      fields.push((field_name, *ty));
    }
    
    let variants = vec![(tuple_name, fields)];
    self.create_tuple_sort(tuple_name, variants)
  }

  fn mk_tuple_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
    assert!(ty.is_tuple() && !ty.is_unit());
    let def = ty.tuple_def();
    let tuple_name = ty.name();
    
    if self.tuple_sorts.contains_key(&tuple_name) {
      return self.tuple_sorts.get(&tuple_name).unwrap().sort.clone();
    }

    let mut fields = Vec::new();
    for (i, ty) in def.iter().enumerate() {
      let field_name = tuple_name + "_" + i.to_string();
      fields.push((field_name, *ty));
    }
    
    let variants = vec![(tuple_name, fields)];
    self.create_tuple_sort(tuple_name, variants)
  }
  
  fn mk_struct(
    &mut self,
    fields: &Vec<z3::ast::Dynamic<'ctx>>,
    ty: Type
  ) -> z3::ast::Dynamic<'ctx> {
    assert!(ty.is_struct());
    let name = NString::from("_struct_") + ty.name();
    if !self.tuple_sorts.contains_key(&name) { self.mk_struct_sort(ty); }
    let dtsort = self.tuple_sorts.get(&name).unwrap();
    let f = &dtsort.variants[0].constructor;
    let mut args = Vec::new();
    for arg in fields.iter() { 
      args.push(arg as &dyn Ast<'_>);
    }
    f.apply(args.as_slice())
  }

  fn mk_tuple(
    &mut self,
    fields: &Vec<z3::ast::Dynamic<'ctx>>,
    ty: Type
  ) -> z3::ast::Dynamic<'ctx> {
    assert!(ty.is_tuple());
    let name = ty.name();
    if !self.tuple_sorts.contains_key(&name) { self.mk_tuple_sort(ty); }
    let dtsort = self.tuple_sorts.get(&name).unwrap();
    let f = &dtsort.variants[0].constructor;
    let mut args = Vec::new();
    for arg in fields.iter() { 
      args.push(arg as &dyn Ast<'_>);
    }
    f.apply(args.as_slice())
  }

  fn mk_tuple_select(
    &mut self,
    object: Expr,
    field: BigInt
  ) -> z3::ast::Dynamic<'ctx> {
    assert!(object.ty().is_tuple() || object.ty().is_struct());
    let mut name = object.ty().name();
    if object.ty().is_struct() { name = NString::from("_struct_") + name; }
    let args = 
      &[&self.convert_ast(object.clone()) as &dyn Ast];
    let dtsort =
      self.tuple_sorts.get(&name)
      .expect(format!("{object:?} is not struct").as_str());
    assert!(field >= BigInt::ZERO);
    assert!(field < dtsort.variants[0].accessors.len().into());
    dtsort
      .variants[0]
      .accessors[bigint_to_usize(&field)]
      .apply(args)
  }
  
  fn mk_tuple_store(
    &mut self,
    object: Expr,
    field: BigInt,
    value: Expr
  ) -> z3::ast::Dynamic<'ctx> {
    assert!(object.ty().is_tuple());
    let mut name = object.ty().name();
    if object.ty().is_struct() { name = NString::from("_struct_") + name; }
    let n = self.tuple_sorts.get(&name).unwrap().variants[0].accessors.len();
    let mut fields_values = Vec::with_capacity(n);
    let update_value = self.convert_ast(value);
    for i in 0..n {
      if field != i.into() {
        fields_values.push(self.mk_tuple_select(object.clone(), i.into()));
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
      .get(&name)
      .unwrap()
      .variants[0]
      .constructor
      .apply(&args.as_slice())
  }
}