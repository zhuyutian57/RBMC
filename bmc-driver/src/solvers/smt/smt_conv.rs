use std::fmt::Debug;

use num_bigint::BigInt;

use crate::expr::constant::Constant;
use crate::expr::expr::*;
use crate::expr::op::*;
use crate::expr::ty::*;
use crate::solvers::solver::PResult;
use crate::symbol::nstring::NString;

pub(crate) trait SmtSolver<'ctx> {
    fn init(&mut self);
    fn assert_assign(&mut self, lhs: Expr, rhs: Expr);
    fn assert_expr(&mut self, expr: Expr);
    fn push(&self);
    fn pop(&self);
    fn reset(&mut self);
    fn check(&self) -> PResult;
    fn eval_bool(&self, expr: Expr) -> bool;
    fn show_model(&self);
}

pub(crate) trait Convert<Sort, Ast: Clone + Debug> {
    fn cache_ast(&mut self, expr: Expr, ast: Ast);
    fn get_cache_ast(&self, expr: &Expr) -> Option<Ast>;
    fn cache_alloc_ast(&mut self, ast: Ast);

    fn convert_sort(&mut self, ty: Type) -> Sort {
        if ty.is_bool() {
            return self.mk_bool_sort();
        }

        if ty.is_integer() {
            return self.mk_int_sort();
        }

        if ty.is_primitive_ptr() {
            return self.mk_pointer_sort();
        }

        if ty.is_array() {
            let domain = self.convert_sort(ty.array_domain());
            let range = self.convert_sort(ty.elem_type());
            return self.mk_array_sort(&domain, &range);
        }

        if ty.is_struct() {
            return self.convert_struct_sort(ty);
        }

        if ty.is_tuple() {
            return self.convert_tuple_sort(ty);
        }

        if ty.is_enum() {
            return self.convert_enum_sort(ty);
        }

        panic!("Not support {:?} yet", ty);
    }

    fn convert_struct_sort(&mut self, ty: Type) -> Sort;
    fn convert_tuple_sort(&mut self, ty: Type) -> Sort;
    fn convert_enum_sort(&mut self, ty: Type) -> Sort;

    fn convert_ast(&mut self, expr: Expr) -> Ast {
        if let Some(a) = self.get_cache_ast(&expr) {
            return a;
        }

        // convert sub exprs firstly
        let mut args: Vec<Ast> = Vec::new();
        if !expr.is_terminal()
            && !expr.is_address_of()
            && !expr.is_index()
            && !expr.is_cast()
            && !expr.is_store()
            && !expr.is_match_variant()
            && !expr.is_as_variant()
        {
            for e in expr.sub_exprs() {
                if e.ty().is_zero_sized_type() {
                    continue;
                }
                args.push(self.convert_ast(e));
            }
        }

        let mut a = None;
        if expr.is_terminal() {
            a = self.convert_terminal(expr.clone());
        }

        if expr.is_address_of() {
            let object = expr.extract_object();
            a = Some(self.convert_address_of(object));
        }

        if expr.is_aggregate() {
            a = if expr.ty().is_struct() {
                Some(self.convert_struct(&args, expr.ty()))
            } else if expr.ty().is_array() {
                Some(self.convert_array(&args, expr.ty()))
            } else {
                Some(self.convert_tuple(&args, expr.ty()))
            };
        }

        if expr.is_binary() {
            let lhs = &args[0];
            let rhs = &args[1];
            a = Some(match expr.extract_bin_op() {
                BinOp::Add => self.mk_add(lhs, rhs),
                BinOp::Sub => self.mk_sub(lhs, rhs),
                BinOp::Mul => self.mk_mul(lhs, rhs),
                BinOp::Div => self.mk_div(lhs, rhs),
                BinOp::Eq => self.mk_eq(lhs, rhs),
                BinOp::Ne => self.mk_ne(lhs, rhs),
                BinOp::Ge => self.mk_ge(lhs, rhs),
                BinOp::Gt => self.mk_gt(lhs, rhs),
                BinOp::Le => self.mk_le(lhs, rhs),
                BinOp::Lt => self.mk_lt(lhs, rhs),
                BinOp::And => self.mk_and(lhs, rhs),
                BinOp::Or => self.mk_or(lhs, rhs),
                BinOp::Implies => self.mk_implies(lhs, rhs),
                BinOp::Offset => {
                    let pt = lhs;
                    let base = self.convert_pointer_base(pt);
                    let o1 = self.convert_pointer_offset(pt);
                    let o2 = rhs;
                    let offset = self.mk_add(&o1, o2);
                    let meta = self.convert_pointer_meta(pt);
                    self.convert_pointer(&base, &offset, Some(&meta))
                }
            });
        }

        if expr.is_unary() {
            a = Some(match expr.extract_un_op() {
                UnOp::Not => self.mk_not(&args[0]),
                _ => panic!("Not support"),
            });
        }

        if expr.is_ite() {
            let cond = &args[0];
            let true_value = &args[1];
            let false_value = &args[2];
            a = Some(self.mk_ite(cond, true_value, false_value));
        }

        if expr.is_cast() {
            a = Some(self.convert_cast(expr.extract_src(), expr.extract_target_type()));
        }

        if expr.is_object() {
            a = Some(args[0].clone());
        }

        if expr.is_same_object() {
            let base_1 = self.project(&args[0]);
            let base_2 = self.project(&args[1]);
            a = Some(self.mk_eq(&base_1, &base_2));
        }

        if expr.is_index() {
            let object = expr.extract_object();
            let index = expr.extract_index();
            a = Some(self.convert_index(object, index));
        }

        if expr.is_store() {
            let object = expr.extract_object();
            let index = expr.extract_index();
            let value = expr.extract_update_value();
            a = Some(self.convert_store(object, index, value));
        }

        if expr.is_pointer() {
            let base = self.convert_pointer_base(&args[0]);
            let offset = self.convert_pointer_offset(&args[0]);
            let meta = &args[1];
            a = Some(self.convert_pointer(&base, &offset, Some(&meta)));
        }

        if expr.is_pointer_base() {
            a = Some(self.convert_pointer_base(&args[0]));
        }

        if expr.is_pointer_offset() {
            a = Some(self.convert_pointer_offset(&args[0]));
        }

        if expr.is_pointer_meta() {
            a = Some(self.convert_pointer_meta(&args[0]));
        }

        if expr.is_variant() {
            let ty = expr.ty();
            let idx = expr.extract_variant_idx();
            let data = if args.len() == 1 { None } else { Some(args[1].clone()) };
            a = Some(self.convert_enum(idx, data, ty))
        }

        if expr.is_as_variant() {
            let x = expr.extract_enum();
            let idx = expr.extract_variant_idx();
            a = Some(self.convert_as_variant(x, idx));
        }

        if expr.is_match_variant() {
            let x = expr.extract_enum();
            let idx = expr.extract_variant_idx();
            a = Some(self.convert_match_variant(x, idx));
        }

        if expr.is_invalid_object() {
            a = Some(self.mk_fresh("Invalid-Object".into(), expr.ty()));
        }

        match a {
            Some(ast) => {
                self.cache_ast(expr, ast.clone());
                ast
            }
            None => panic!("Not implememt: {expr:?}"),
        }
    }

    fn convert_terminal(&mut self, expr: Expr) -> Option<Ast> {
        let mut a = None;
        if expr.is_constant() {
            a = self.convert_constant(&expr.extract_constant(), expr.ty());
        }

        if expr.is_symbol() {
            let sym = expr.extract_symbol();
            let symbol = self.convert_symbol(sym.name(), expr.ty());

            if sym.ident().to_nstring() == NString::ALLOC_SYM {
                self.cache_alloc_ast(symbol.clone());
            }

            a = Some(symbol);
        }

        a
    }

    fn convert_constant(&mut self, constant: &Constant, ty: Type) -> Option<Ast> {
        match constant {
            Constant::Bool(b) => Some(self.mk_smt_bool(*b)),
            Constant::Integer(i) => Some(self.mk_smt_int(i.clone())),
            Constant::Null(ty) => Some(self.convert_null(*ty)),
            Constant::Array(c, t) => {
                let domain = self.convert_sort(ty.array_domain());
                let val = self.convert_constant(&**c, *t).unwrap();
                Some(self.mk_smt_const_array(&domain, &val))
            }
            Constant::Adt(constants, t) => {
                assert!(ty == *t);
                if ty.is_struct() || ty.is_tuple() {
                    let mut fields = Vec::new();
                    for i in 0..ty.fields() {
                        if ty.field_type(i).is_zero_sized_type() {
                            continue;
                        }
                        let field = self.convert_constant(&constants[i], ty.field_type(i)).unwrap();
                        fields.push(field);
                    }
                    if ty.is_struct() {
                        Some(self.convert_struct(&fields, ty))
                    } else {
                        Some(self.convert_tuple(&fields, ty))
                    }
                } else {
                    todo!("{ty:?}")
                }
            }
            Constant::Zst(_) => None,
        }
    }

    fn convert_null(&self, ty: Type) -> Ast;
    fn convert_pointer(&self, base: &Ast, offset: &Ast, meta: Option<&Ast>) -> Ast;
    fn convert_pointer_base(&self, pt: &Ast) -> Ast;
    fn convert_pointer_offset(&self, pt: &Ast) -> Ast;
    fn convert_pointer_meta(&self, pt: &Ast) -> Ast;

    fn convert_array(&mut self, elem: &Vec<Ast>, ty: Type) -> Ast {
        let mut array = self.mk_fresh("array".into(), ty);
        for (i, val) in elem.iter().enumerate() {
            let index = self.mk_smt_int(BigInt::from(i));
            array = self.mk_store(&array, &index, val);
        }
        array
    }

    fn convert_struct(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn convert_tuple(&mut self, fields: &Vec<Ast>, ty: Type) -> Ast;
    fn convert_enum(&mut self, idx: usize, data: Option<Ast>, ty: Type) -> Ast;

    fn convert_symbol(&mut self, name: NString, ty: Type) -> Ast {
        if ty.is_bool() {
            return self.mk_bool_symbol(name);
        }
        if ty.is_integer() {
            return self.mk_int_symbol(name);
        }
        if ty.is_primitive_ptr() {
            let sort = self.convert_sort(ty);
            return self.mk_tuple_symbol(name, &sort);
        }
        if ty.is_array() {
            let domain = self.convert_sort(ty.array_domain());
            let range = self.convert_sort(ty.elem_type());
            return self.mk_array_symbol(name, &domain, &range);
        }
        if ty.is_struct() {
            let sort = self.convert_struct_sort(ty);
            return self.mk_tuple_symbol(name, &sort);
        }
        if ty.is_tuple() {
            let sort = self.convert_enum_sort(ty);
            return self.mk_tuple_symbol(name, &sort);
        }
        if ty.is_enum() {
            let sort = self.convert_enum_sort(ty);
            return self.mk_enum_symbol(name, &sort);
        }
        panic!("{name:?} {ty:?} symbol is not support?")
    }

    fn convert_address_of(&mut self, object: Expr) -> Ast {
        assert!(object.is_object());
        let inner_expr = object.extract_inner_expr();
        if inner_expr.is_index() {
            let inner_object = inner_expr.extract_object();
            let inner_offset = inner_expr.extract_index();
            let base = self.convert_object_space(&inner_object);
            let offset = self.convert_ast(inner_offset);
            return self.convert_pointer(&base, &offset, None);
        }

        if inner_expr.is_symbol() {
            let base = self.convert_object_space(&object);
            let offset = self.mk_smt_int(BigInt::ZERO);
            return self.convert_pointer(&base, &offset, None);
        }

        if inner_expr.is_slice() {
            let inner_object = inner_expr.extract_object();
            let base = self.convert_object_space(&inner_object);
            let start = inner_expr.extract_slice_start();
            let offset = self.convert_ast(start);
            let len = inner_expr.extract_slice_len();
            let meta = self.convert_ast(len);
            return self.convert_pointer(&base, &offset, Some(&meta));
        }

        panic!("Do not support address_of {object:?}")
    }

    fn convert_object_space(&mut self, object: &Expr) -> Ast;

    fn convert_cast(&mut self, expr: Expr, target_ty: Type) -> Ast {
        if expr.ty().is_integer() && target_ty.is_integer() {
            return self.convert_ast(expr.clone());
        }

        if expr.ty().is_primitive_ptr() {
            return self.convert_cast_from_ptr(expr, target_ty);
        }

        panic!("Do not support cast {:?} to {target_ty:?}", expr.ty())
    }

    fn convert_cast_from_ptr(&mut self, pt: Expr, target_ty: Type) -> Ast {
        if pt.ty().is_primitive_ptr() {
            if target_ty.is_primitive_ptr() {
                return self.convert_ast(pt);
            }

            // cast pointer to integer
            if target_ty.is_integer() {
                let pointer_base = pt.ctx.pointer_base(pt.clone());
                let pointer_offset = pt.ctx.pointer_offset(pt.clone());
                let base = self.convert_ast(pointer_base);
                let offset = self.convert_ast(pointer_offset);
                return self.mk_add(&base, &offset);
            }
        }

        panic!("Do not support cast {:?} to {target_ty:?}", pt.ty())
    }

    fn convert_index(&mut self, object: Expr, index: Expr) -> Ast {
        if object.ty().is_array() {
            let array = self.convert_ast(object.clone());
            let i = self.convert_ast(index.clone());
            return self.mk_select(&array, &i);
        }

        if object.ty().is_struct() || object.ty().is_tuple() {
            return self.convert_index_tuple(object.clone(), index.clone());
        }

        if object.ty().is_enum() {
            return self.convert_index_enum(object.clone(), index.clone());
        }

        panic!("Do not support load {object:?} with {:?}", object.ty())
    }

    fn convert_index_tuple(&mut self, object: Expr, field: Expr) -> Ast;
    fn convert_index_enum(&mut self, object: Expr, field: Expr) -> Ast;

    fn convert_store(&mut self, object: Expr, index: Expr, value: Expr) -> Ast {
        if object.ty().is_array() {
            let array = self.convert_ast(object.clone());
            let i = if object.ty().array_domain() != index.ty() {
                self.convert_cast(index, object.ty().array_domain())
            } else {
                self.convert_ast(index.clone())
            };
            let val = self.convert_ast(value.clone());
            return self.mk_store(&array, &i, &val);
        }

        if object.ty().is_struct() || object.ty().is_tuple() {
            return self.convert_tuple_update(object.clone(), index.clone(), value.clone());
        }

        if object.ty().is_enum() {
            let inner_expr = object.extract_inner_expr();
            assert!(inner_expr.is_as_variant());
            return self.convert_variant_update(inner_expr, index.clone(), value.clone());
        }

        panic!("Do not support store {object:?} with {:?}", object.ty())
    }

    fn convert_tuple_update(&mut self, object: Expr, field: Expr, value: Expr) -> Ast;
    fn convert_variant_update(&mut self, variant: Expr, field: Expr, value: Expr) -> Ast;

    fn convert_as_variant(&mut self, _enum: Expr, idx: usize) -> Ast;
    fn convert_match_variant(&mut self, _enum: Expr, idx: usize) -> Ast;

    // fresh variable
    fn mk_fresh(&mut self, prefix: NString, ty: Type) -> Ast;

    // sort
    fn mk_bool_sort(&self) -> Sort;
    fn mk_int_sort(&self) -> Sort;
    fn mk_array_sort(&mut self, domain: &Sort, range: &Sort) -> Sort;
    fn mk_pointer_sort(&self) -> Sort;

    // constant
    fn mk_smt_bool(&self, b: bool) -> Ast;
    fn mk_smt_int(&self, i: BigInt) -> Ast;
    fn mk_smt_const_array(&self, domain: &Sort, val: &Ast) -> Ast;

    // symbol
    fn mk_bool_symbol(&self, name: NString) -> Ast;
    fn mk_int_symbol(&self, name: NString) -> Ast;
    fn mk_array_symbol(&self, name: NString, domain: &Sort, range: &Sort) -> Ast;
    fn mk_tuple_symbol(&self, name: NString, sort: &Sort) -> Ast;
    fn mk_enum_symbol(&self, name: NString, sort: &Sort) -> Ast;

    // pointer
    fn project(&self, pt: &Ast) -> Ast;

    // array
    fn mk_select(&self, array: &Ast, index: &Ast) -> Ast;
    fn mk_store(&self, array: &Ast, index: &Ast, val: &Ast) -> Ast;

    // expr
    fn mk_add(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_sub(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_mul(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_div(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_eq(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_ne(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_ge(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_gt(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_le(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_lt(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_and(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_or(&self, lhs: &Ast, rhs: &Ast) -> Ast;
    fn mk_not(&self, operand: &Ast) -> Ast;
    fn mk_implies(&self, cond: &Ast, conseq: &Ast) -> Ast;
    fn mk_ite(&self, cond: &Ast, true_value: &Ast, false_value: &Ast) -> Ast;
}
