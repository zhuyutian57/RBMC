
use num_bigint::BigInt;
use z3::ast::*;
use z3::DatatypeAccessor;

use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::NString;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_memspace::*;
use super::z3_conv::*;

impl<'ctx> MemSpace<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
  fn set_pointer_logic(&mut self) {
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
    self.tuple_sorts.insert(NString::from("pointer"), pointer_tuple_sort);

    // General pointer sort
    let pointer_sort = self.pointer_sort();

    // A box pointer is a tuple (pointer)
    let pointer_tuple_sort =
      z3::DatatypeBuilder
        ::new(&self.z3_ctx, "box")
        .variant(
          "box",
          vec![
            ("box_ptr", DatatypeAccessor::Sort(pointer_sort.clone())),
            ]
          )
        .finish();
    self.tuple_sorts.insert(NString::from("box"), pointer_tuple_sort);

    // A slice pointer is a tuple (pointer, len)
    let pointer_tuple_sort =
    z3::DatatypeBuilder
      ::new(&self.z3_ctx, "slice")
      .variant(
        "slice",
        vec![
          ("slice_ptr", DatatypeAccessor::Sort(pointer_sort)),
          ("slice_meta", DatatypeAccessor::Sort(z3::Sort::int(&self.z3_ctx)))
          ]
        )
      .finish();
    self.tuple_sorts.insert(NString::from("slice"), pointer_tuple_sort);
  }

  fn pointer_sort(&self) -> z3::Sort<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("pointer"))
      .expect("Pointer tuple is not initialized")
      .sort
      .clone()
  }

  fn box_sort(&self) -> z3::Sort<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("box"))
      .expect("Box pointer tuple is not initialized")
      .sort
      .clone()
  }

  fn slice_ptr_sort(&self) -> z3::Sort<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("slice"))
      .expect("Slice pointer tuple is not initialized")
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

    // Use l0 as identifier
    let space_base =
      NString::from(object.extract_symbol().ident()) + "_base";
    // The size is field-level
    let space_len =
      if object.ty().is_struct() {
        object.ty().struct_def().1.len()
      } else if object.ty().is_array() {
        object.ty().array_size().expect("Array must have size") as usize
      } else { 1 };

    let base = self.mk_int_symbol(space_base);
    let len = self.mk_smt_int(BigInt::from(space_len));

    // base is greater than 0
    self.assert(self.mk_gt(&base, &self.mk_smt_int(BigInt::ZERO)));
    // disjoint relationship
    for (b, l)
      in self.pointer_logic.object_spaces().values() {
      if space_base == NString::from(b.to_string()) { continue; }
      
      assert!(self.cur_alloc_expr != None);
      let alloc_array_ast = self.cur_alloc_expr.as_ref().unwrap();
      let alive = alloc_array_ast.as_array().unwrap().select(b);
      
      let l1 = base.clone();
      let r1 = self.mk_add(&l1, &len);
      let l2 = b.clone();
      let r2 = self.mk_add(&l2, &l);
      let no_overlap =
        self.mk_or(
          &self.mk_le(&r1, &l2),
          &self.mk_le(&r2, &l1)
        );
      let disj = self.mk_implies(&alive,&no_overlap);
      self.assert(disj);
    }
    
    self.pointer_logic.set_object_space(object.clone(), (base, len));
  }

  fn mk_pointer(
    &self,
    base: &z3::ast::Dynamic<'ctx>,
    offset: &z3::ast::Dynamic<'ctx>
  ) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("pointer"))
      .unwrap()
      .variants[0]
      .constructor
      .apply(&[base as &dyn Ast, offset as &dyn Ast])
  }

  fn mk_pointer_ident(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("pointer"))
      .unwrap()
      .variants[0]
      .accessors[0]
      .apply(&[pt as &dyn Ast])
  }

  fn mk_pointer_offset(&self, pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("pointer"))
      .unwrap()
      .variants[0]
      .accessors[1]
      .apply(&[pt as &dyn Ast])
  }
  
  fn mk_box(&self, inner_pt: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("box"))
      .unwrap()
      .variants[0]
      .constructor
      .apply(&[inner_pt as &dyn Ast])
  }
  
  fn mk_box_ptr(&self, _box: &z3::ast::Dynamic<'ctx>) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("box"))
      .unwrap()
      .variants[0]
      .accessors[0]
      .apply(&[_box as &dyn Ast])
  }
  
  fn mk_slice(
    &self,
    pt: &z3::ast::Dynamic<'ctx>,
    meta: &z3::ast::Dynamic<'ctx>
  ) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("slice"))
      .unwrap()
      .variants[0]
      .constructor
      .apply(&[pt as &dyn Ast, meta as &dyn Ast])
  }

  fn mk_slice_ptr(
    &self,
    slice: &z3::ast::Dynamic<'ctx>
  ) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("slice"))
      .unwrap()
      .variants[0]
      .accessors[0]
      .apply(&[slice as &dyn Ast])
  }

  fn mk_slice_meta(
    &self,
    slice: &z3::ast::Dynamic<'ctx>
  ) -> z3::ast::Dynamic<'ctx> {
    self
      .tuple_sorts
      .get(&NString::from("slice"))
      .unwrap()
      .variants[0]
      .accessors[1]
      .apply(&[slice as &dyn Ast])
  }
}