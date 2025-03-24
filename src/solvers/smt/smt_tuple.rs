use num_bigint::BigInt;

use crate::expr::expr::Expr;
use crate::expr::ty::*;
use crate::symbol::nstring::NString;

pub type Variant = Vec<FieldDef>;
pub type Variants = Vec<(NString, Variant)>;

pub trait Tuple<Sort, Ast> {
    fn create_tuple_sort(&mut self, name: NString, variants: Variants) -> Sort;
    fn mk_struct_sort(&mut self, ty: Type) -> Sort;
    fn mk_tuple_sort(&mut self, ty: Type) -> Sort;
    fn mk_struct(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn mk_tuple(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn mk_tuple_select(&mut self, object: Expr, field: BigInt) -> Ast;
    fn mk_tuple_store(&mut self, object: Expr, field: BigInt, value: Expr) -> Ast;
}
