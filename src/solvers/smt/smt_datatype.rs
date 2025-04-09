use num_bigint::BigInt;

use crate::expr::expr::Expr;
use crate::expr::ty::*;
use crate::symbol::nstring::NString;

pub trait DataType<Sort, Ast> {
    fn create_datatype(&mut self, name: NString, variants: Variants) -> Sort;
    fn mk_struct_sort(&mut self, ty: Type) -> Sort;
    fn mk_tuple_sort(&mut self, ty: Type) -> Sort;
    fn mk_enum_sort(&mut self, ty: Type) -> Sort;
    fn mk_struct(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn mk_tuple(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn mk_tuple_select(&mut self, object: Expr, field: BigInt) -> Ast;
    fn mk_tuple_store(&mut self, object: Expr, field: BigInt, value: Expr) -> Ast;
    fn mk_variant(&mut self, idx: usize, data: Option<Ast>, ty: Type) -> Ast;
    fn mk_match_variant(&mut self, _enum: Expr, idx: usize) -> Ast;
}
