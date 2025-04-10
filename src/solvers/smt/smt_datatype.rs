use num_bigint::BigInt;

use crate::expr::expr::Expr;
use crate::expr::ty::*;
use crate::symbol::nstring::NString;

/// Use type name and its parameter to identify a datatype.
/// Handling the generic type
pub type DataTypeSign = (NString, Vec<Type>);

pub trait DataType<Sort, Ast> {
    fn create_datatype_sign(&mut self, ty: Type) -> DataTypeSign;
    fn create_datatype(&mut self, sign: DataTypeSign, variants: Variants) -> Sort;
    fn mk_struct_sort(&mut self, ty: Type) -> Sort;
    fn mk_tuple_sort(&mut self, ty: Type) -> Sort;
    fn mk_enum_sort(&mut self, ty: Type) -> Sort;
    fn mk_struct(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn mk_tuple(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn mk_tuple_select(&mut self, object: Ast, field: usize, ty: Type) -> Ast;
    fn mk_tuple_store(&mut self, object: Ast, field: usize, value: Ast, ty: Type) -> Ast;
    fn mk_variant(&mut self, idx: usize, data: Option<Ast>, ty: Type) -> Ast;
    fn mk_as_variant(&mut self, _enum: Expr, idx: usize) -> Ast;
    fn mk_match_variant(&mut self, _enum: Expr, idx: usize) -> Ast;
}
