use std::collections::HashSet;
use std::fmt::Debug;
use std::fmt::Error;
use std::hash::Hash;

use num_bigint::BigInt;
use stable_mir::mir::Mutability;

use super::ast::*;
use super::constant::*;
use super::context::*;
use super::op::*;
use super::ty::*;
use crate::program::program::bigint_to_usize;
use crate::symbol::nstring::NString;
use crate::symbol::symbol::*;

/// `Expr` is a wrapper for AST node. It only carry node index that
/// is used to construct AST. The corresponding information can be
/// retrieved from `Context`
#[derive(Clone)]
pub struct Expr {
    pub ctx: ExprCtx,
    pub(super) id: NodeId,
}

impl Expr {
    pub fn ty(&self) -> Type {
        self.ctx.borrow().ty(self.id)
    }

    pub fn is_terminal(&self) -> bool {
        self.ctx.borrow().is_terminal(self.id)
    }
    pub fn is_true(&self) -> bool {
        self.ctx.borrow().is_true(self.id)
    }
    pub fn is_false(&self) -> bool {
        self.ctx.borrow().is_false(self.id)
    }
    pub fn is_constant(&self) -> bool {
        self.ctx.borrow().is_constant(self.id)
    }
    pub fn is_null(&self) -> bool {
        self.ctx.borrow().is_null(self.id)
    }
    pub fn is_type(&self) -> bool {
        self.ctx.borrow().is_type(self.id)
    }
    pub fn is_symbol(&self) -> bool {
        self.ctx.borrow().is_symbol(self.id)
    }

    pub fn is_address_of(&self) -> bool {
        self.ctx.borrow().is_address_of(self.id)
    }
    pub fn is_aggregate(&self) -> bool {
        self.ctx.borrow().is_aggregate(self.id)
    }
    pub fn is_binary(&self) -> bool {
        self.ctx.borrow().is_binary(self.id)
    }
    pub fn is_unary(&self) -> bool {
        self.ctx.borrow().is_unary(self.id)
    }
    pub fn is_ite(&self) -> bool {
        self.ctx.borrow().is_ite(self.id)
    }
    pub fn is_cast(&self) -> bool {
        self.ctx.borrow().is_cast(self.id)
    }
    pub fn is_object(&self) -> bool {
        self.ctx.borrow().is_object(self.id)
    }
    pub fn is_slice(&self) -> bool {
        self.ctx.borrow().is_slice(self.id)
    }
    pub fn is_same_object(&self) -> bool {
        self.ctx.borrow().is_same_object(self.id)
    }
    pub fn is_index_non_zero(&self) -> bool {
        self.ctx.borrow().is_index_non_zero(self.id)
    }
    pub fn is_index_zero_sized(&self) -> bool {
        self.ctx.borrow().is_index_zero_sized(self.id)
    }
    pub fn is_store(&self) -> bool {
        self.ctx.borrow().is_store(self.id)
    }

    pub fn is_offset(&self) -> bool {
        self.ctx.borrow().is_offset(self.id)
    }
    pub fn is_pointer_base(&self) -> bool {
        self.ctx.borrow().is_pointer_base(self.id)
    }
    pub fn is_pointer_offset(&self) -> bool {
        self.ctx.borrow().is_pointer_offset(self.id)
    }
    pub fn is_pointer_meta(&self) -> bool {
        self.ctx.borrow().is_pointer_meta(self.id)
    }
    pub fn is_vec(&self) -> bool {
        self.ctx.borrow().is_vec(self.id)
    }
    pub fn is_vec_len(&self) -> bool {
        self.ctx.borrow().is_vec_len(self.id)
    }
    pub fn is_vec_cap(&self) -> bool {
        self.ctx.borrow().is_vec_cap(self.id)
    }
    pub fn is_inner_pointer(&self) -> bool {
        self.ctx.borrow().is_inner_pointer(self.id)
    }

    pub fn is_enum(&self) -> bool {
        self.ctx.borrow().is_enum(self.id)
    }
    pub fn is_as_variant(&self) -> bool {
        self.ctx.borrow().is_as_variant(self.id)
    }
    pub fn is_match_variant(&self) -> bool {
        self.ctx.borrow().is_match_variant(self.id)
    }

    pub fn is_move(&self) -> bool {
        self.ctx.borrow().is_move(self.id)
    }
    pub fn is_valid(&self) -> bool {
        self.ctx.borrow().is_valid(self.id)
    }
    pub fn is_invalid(&self) -> bool {
        self.ctx.borrow().is_invalid(self.id)
    }
    pub fn is_null_object(&self) -> bool {
        self.ctx.borrow().is_null_object(self.id)
    }
    pub fn is_unknown(&self) -> bool {
        self.ctx.borrow().is_unknown(self.id)
    }

    pub fn extract_symbol(&self) -> Symbol {
        self.ctx.borrow().extract_symbol(self.id).expect("Not symbol")
    }

    pub fn extract_constant(&self) -> Constant {
        self.ctx.borrow().extract_constant(self.id).expect("Not constant")
    }

    pub fn extract_integer(&self) -> BigInt {
        self.extract_constant().to_integer()
    }

    pub fn extract_struct_fields(&self) -> Vec<ConstantField> {
        self.extract_constant().to_struct_fields()
    }

    pub fn extract_type(&self) -> Type {
        self.ctx.borrow().extract_type(self.id).unwrap()
    }

    pub fn extract_address_type(&self) -> Type {
        assert!(self.is_object());
        Type::ptr_type(self.ty(), Mutability::Not)
    }

    pub fn extract_object(&self) -> Expr {
        assert!(
            self.is_address_of()
                || self.is_slice()
                || self.is_index_non_zero()
                || self.is_index_zero_sized()
                || self.is_store()
                || self.is_move()
                || self.is_valid()
                || self.is_invalid()
        );
        self.extract_sub_expr(0)
    }

    pub fn extract_root_object(&self) -> Expr {
        if self.is_symbol() {
            return self.ctx.object(self.clone());
        }

        if self.is_object() {
            return self.extract_inner_expr().extract_root_object();
        }

        if self.is_address_of() || self.is_slice() || self.is_index_non_zero() || self.is_store() {
            return self.extract_object().extract_root_object();
        }

        panic!("Impossible")
    }

    pub fn extract_fields(&self) -> Vec<Expr> {
        assert!(self.is_aggregate());
        self.sub_exprs().unwrap()
    }

    pub fn extract_bin_op(&self) -> BinOp {
        assert!(self.is_binary());
        self.ctx.borrow().extract_bin_op(self.id).unwrap()
    }

    pub fn extract_lhs(&self) -> Expr {
        assert!(self.is_binary() || self.is_same_object());
        self.extract_sub_expr(0)
    }

    pub fn extract_rhs(&self) -> Expr {
        assert!(self.is_binary() || self.is_same_object());
        self.extract_sub_expr(1)
    }

    pub fn extract_root_pointer(&self) -> Expr {
        assert!(self.ty().is_ptr());
        if self.is_offset() {
            self.extract_inner_pointer().extract_root_pointer()
        } else {
            self.clone()
        }
    }

    /// This function will compute the total offset expr.
    pub fn extract_offset(&self) -> Expr {
        assert!(self.ty().is_ptr() && self.is_offset());
        let pt = self.extract_inner_pointer();
        let r_offset = self.extract_sub_expr(1);
        let l_offset = if pt.is_offset() {
            pt.extract_offset()
        } else {
            self.ctx.constant_integer(BigInt::ZERO, r_offset.ty())
        };
        let mut offset = self.ctx.add(l_offset, r_offset);
        offset.simplify();
        offset
    }

    pub fn extract_un_op(&self) -> UnOp {
        assert!(self.is_unary());
        self.ctx.borrow().extract_un_op(self.id).unwrap()
    }

    pub fn extract_src(&self) -> Expr {
        assert!(self.is_cast());
        self.extract_sub_expr(0)
    }

    pub fn extract_target_type(&self) -> Type {
        assert!(self.is_cast());
        self.extract_sub_expr(1).extract_type()
    }

    pub fn extract_cond(&self) -> Expr {
        assert!(self.is_ite());
        self.extract_sub_expr(0)
    }

    pub fn extract_true_value(&self) -> Expr {
        assert!(self.is_ite());
        self.extract_sub_expr(1)
    }

    pub fn extract_false_value(&self) -> Expr {
        assert!(self.is_ite());
        self.extract_sub_expr(2)
    }

    pub fn extract_inner_expr(&self) -> Expr {
        assert!(self.is_object() || self.is_unary());
        self.extract_sub_expr(0)
    }

    pub fn extract_slice_start(&self) -> Expr {
        assert!(self.is_slice());
        self.extract_sub_expr(1)
    }

    pub fn extract_slice_len(&self) -> Expr {
        assert!(self.is_slice());
        self.extract_sub_expr(2)
    }

    pub fn extract_index(&self) -> Expr {
        assert!(self.is_index_non_zero() || self.is_store());
        self.extract_sub_expr(1)
    }

    pub fn extract_update_value(&self) -> Expr {
        assert!(self.is_store());
        self.extract_sub_expr(2)
    }

    pub fn extract_vec_len(&self) -> Expr {
        assert!(self.is_vec());
        self.extract_sub_expr(1)
    }

    pub fn extract_vec_cap(&self) -> Expr {
        assert!(self.is_vec());
        self.extract_sub_expr(2)
    }

    pub fn extract_inner_pointer(&self) -> Expr {
        assert!(
            self.is_offset()
                || self.is_pointer_base()
                || self.is_pointer_offset()
                || self.is_pointer_meta()
                || self.is_vec()
                || self.is_vec_len()
                || self.is_vec_cap()
                || self.is_inner_pointer()
        );
        self.extract_sub_expr(0)
    }

    pub fn extract_enum(&self) -> Expr {
        assert!(self.is_as_variant() || self.is_match_variant());
        self.extract_sub_expr(0)
    }

    pub fn extract_variant_idx(&self) -> usize {
        assert!(self.is_enum() || self.is_as_variant() || self.is_match_variant());
        let i = if self.is_enum() {
            self.extract_sub_expr(0).extract_constant().to_integer()
        } else {
            self.extract_sub_expr(1).extract_constant().to_integer()
        };
        bigint_to_usize(&i)
    }

    /// Compute offset from root object. The final offset is in byte-level.
    pub fn compute_offset(&self) -> Expr {
        if self.is_symbol() {
            return self.ctx.constant_isize(0);
        }

        if self.is_object() {
            return self.extract_inner_expr().compute_offset();
        }

        if self.is_slice() {
            let mut offset = self.extract_object().compute_offset();
            let elem_size = self.ty().elem_type().size();
            let start = self.extract_slice_start();
            offset = self
                .ctx
                .add(offset, self.ctx.mul(self.ctx.constant_isize(elem_size as isize), start));
            offset.simplify();
            return offset;
        }

        if self.is_index_non_zero() {
            let inner_object = self.extract_object();
            let ty = inner_object.ty();
            let mut offset = inner_object.compute_offset();
            let index = self.extract_index();

            let collected_offset = if ty.is_array() || ty.is_slice() {
                let elem_size = ty.elem_type().size();
                self.ctx.mul(index, self.ctx.constant_usize(elem_size))
            } else {
                assert!(ty.is_struct() || ty.is_tuple());
                assert!(index.is_constant());
                let idx = index.extract_constant().to_integer();
                assert!(BigInt::ZERO <= idx && idx < ty.fields().into());
                let mut i = bigint_to_usize(&idx);
                ty.fix_index_field(&mut i);
                let align = ty.align();
                self.ctx.constant_usize(i * align)
            };
            offset = self.ctx.add(offset, collected_offset);
            offset.simplify();
            return offset;
        }

        // If a field is zero-sized type, return the end of the object.
        if self.is_index_zero_sized() {
            let inner_object = self.extract_object();
            let mut offset = self.ctx.add(
                inner_object.compute_offset(),
                self.ctx.constant_usize(inner_object.ty().size())
            );
            offset.simplify();
            return offset;
        }

        todo!("{self:?}")
    }

    pub fn unwrap_predicates(&self) -> Expr {
        if self.is_move() || self.is_invalid() {
            self.extract_object().unwrap_predicates()
        } else {
            self.clone()
        }
    }

    fn extract_sub_expr(&self, i: usize) -> Expr {
        let sub_exprs = self.sub_exprs().expect("Must be non-empty");
        assert!(i < sub_exprs.len());
        sub_exprs[i].clone()
    }

    /// Construct sub-exprs from AST
    pub fn sub_exprs(&self) -> Option<Vec<Expr>> {
        match self.ctx.borrow().sub_nodes(self.id) {
            Some(ids) => {
                let mut sub_exprs = Vec::new();
                for id in ids {
                    sub_exprs.push(Expr { ctx: self.ctx.clone(), id });
                }
                Some(sub_exprs)
            }
            None => None,
        }
    }

    pub fn has_predicates(&self) -> bool {
        if self.is_invalid() | self.is_move() {
            return true;
        }
        match self.sub_exprs() {
            Some(sub_exprs) => sub_exprs.iter().fold(false, |res, x| res | x.has_predicates()),
            None => false,
        }
    }

    pub fn replace_sub_exprs(&mut self, sub_exprs: Vec<Expr>) {
        if self.is_terminal() || self.is_unknown() {
            return;
        }

        if self.is_address_of() {
            let object = sub_exprs[0].clone();
            *self = self.ctx.address_of(object, self.ty());
            return;
        }

        if self.is_aggregate() {
            *self = self.ctx.aggregate(sub_exprs.clone(), self.ty());
            return;
        }

        if self.is_binary() {
            let lhs = sub_exprs[0].clone();
            let rhs = sub_exprs[1].clone();
            *self = match self.extract_bin_op() {
                BinOp::Add => self.ctx.add(lhs, rhs),
                BinOp::Sub => self.ctx.sub(lhs, rhs),
                BinOp::Mul => self.ctx.mul(lhs, rhs),
                BinOp::Div => self.ctx.div(lhs, rhs),
                BinOp::Eq => self.ctx.eq(lhs, rhs),
                BinOp::Ne => self.ctx.ne(lhs, rhs),
                BinOp::Ge => self.ctx.ge(lhs, rhs),
                BinOp::Gt => self.ctx.gt(lhs, rhs),
                BinOp::Le => self.ctx.le(lhs, rhs),
                BinOp::Lt => self.ctx.lt(lhs, rhs),
                BinOp::And => self.ctx.and(lhs, rhs),
                BinOp::Or => self.ctx.or(lhs, rhs),
                BinOp::Implies => self.ctx.implies(lhs, rhs),
            };
            return;
        }

        if self.is_unary() {
            let operand = sub_exprs[0].clone();
            *self = match self.extract_un_op() {
                UnOp::Not => self.ctx.not(operand),
                UnOp::Neg => self.ctx.neg(operand),
                UnOp::Meta => self.ctx.pointer_meta(operand),
            };
            return;
        }

        if self.is_cast() {
            let operand = sub_exprs[0].clone();
            let target_ty = sub_exprs[1].clone();
            *self = self.ctx.cast(operand, target_ty);
            return;
        }

        if self.is_object() {
            let inner_expr = sub_exprs[0].clone();
            *self = self.ctx.object(inner_expr);
            return;
        }

        if self.is_slice() {
            let object = sub_exprs[0].clone();
            let start = sub_exprs[1].clone();
            let end = sub_exprs[2].clone();
            *self = self.ctx.slice(object, start, end);
            return;
        }

        if self.is_index_non_zero() {
            let object = sub_exprs[0].clone();
            let index = sub_exprs[1].clone();
            *self = self.ctx.index_non_zero(object, index, self.ty());
            return;
        }

        if self.is_index_zero_sized() {
            let object = sub_exprs[0].clone();
            *self = self.ctx.index_zero_sized(object, self.ty());
            return;
        }

        if self.is_ite() {
            let cond = sub_exprs[0].clone();
            let true_value = sub_exprs[1].clone();
            let false_value = sub_exprs[2].clone();
            *self = self.ctx.ite(cond, true_value, false_value);
            return;
        }

        if self.is_same_object() {
            let lhs = sub_exprs[0].clone();
            let rhs = sub_exprs[1].clone();
            *self = self.ctx.same_object(lhs, rhs);
            return;
        }

        if self.is_store() {
            let object = sub_exprs[0].clone();
            let index = sub_exprs[1].clone();
            let value = sub_exprs[2].clone();
            *self = self.ctx.store(object, index, value);
            return;
        }

        if self.is_offset() {
            let pt = sub_exprs[0].clone();
            let offset = sub_exprs[1].clone();
            *self = self.ctx.offset(pt, offset);
            return;
        }

        if self.is_pointer_base() {
            let pt = sub_exprs[0].clone();
            *self = self.ctx.pointer_base(pt);
            return;
        }

        if self.is_pointer_offset() {
            let pt = sub_exprs[0].clone();
            *self = self.ctx.pointer_offset(pt);
            return;
        }

        if self.is_pointer_meta() {
            let pt = sub_exprs[0].clone();
            *self = self.ctx.pointer_meta(pt);
            return;
        }

        if self.is_vec() {
            let pt = sub_exprs[0].clone();
            let len = sub_exprs[1].clone();
            let cap = sub_exprs[2].clone();
            *self = self.ctx._vec(pt, len, cap, self.ty());
            return;
        }

        if self.is_vec_len() {
            let pt = sub_exprs[0].clone();
            *self = self.ctx.vec_len(pt);
            return;
        }

        if self.is_vec_cap() {
            let pt = sub_exprs[0].clone();
            *self = self.ctx.vec_cap(pt);
            return;
        }

        if self.is_inner_pointer() {
            let pt = sub_exprs[0].clone();
            *self = self.ctx.inner_pointer(pt);
            return;
        }

        if self.is_enum() {
            if sub_exprs.len() == 2 {
                let idx = sub_exprs[0].clone();
                let data = sub_exprs[1].clone();
                *self = self.ctx.variant(idx, Some(data), self.ty());
            }
            return;
        }

        if self.is_as_variant() {
            let x = sub_exprs[0].clone();
            let idx = sub_exprs[1].clone();
            *self = self.ctx.as_variant(x, idx);
            return;
        }

        if self.is_match_variant() {
            let x = sub_exprs[0].clone();
            let idx = sub_exprs[1].clone();
            *self = self.ctx.match_variant(x, idx);
            return;
        }

        panic!("Need implementing for {self:?}");
    }

    pub fn unwrap_and(&self) -> HashSet<Expr> {
        let mut s = HashSet::new();
        if self.is_binary() {
            if self.extract_bin_op() == BinOp::And {
                for e in self.extract_lhs().unwrap_and() {
                    s.insert(e);
                }
                for e in self.extract_rhs().unwrap_and() {
                    s.insert(e);
                }
            } else {
                s.insert(self.clone());
            }
        } else {
            s.insert(self.clone());
        }
        s
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Expr {}

impl Hash for Expr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_terminal() {
            write!(f, "{:?}", self.ctx.borrow().extract_terminal(self.id).unwrap())
        } else {
            let sub_exprs = self.sub_exprs().unwrap();

            if self.is_address_of() {
                let place = &sub_exprs[0];
                return write!(f, "&{place:?}");
            }

            if self.is_aggregate() {
                let ty = self.ty();
                let type_info = if ty.is_struct() || ty.is_array() {
                    ty.name()
                } else if ty.is_tuple() {
                    NString::from("Tuple")
                } else {
                    todo!("{ty:?}")
                };
                return write!(f, "{type_info:?} {sub_exprs:?}");
            }

            if self.is_binary() {
                let lhs = &sub_exprs[0];
                let rhs = &sub_exprs[1];
                return write!(f, "({lhs:?} {:?} {rhs:?})", self.extract_bin_op());
            }

            if self.is_unary() {
                return write!(f, "{:?}({:?})", self.extract_un_op(), sub_exprs[0]);
            }

            if self.is_cast() {
                let lhs = &sub_exprs[0];
                let ty = &sub_exprs[1];
                return write!(f, "{lhs:?} as {ty:?}");
            }

            if self.is_object() {
                return write!(f, "{:?}", sub_exprs[0]);
            }

            if self.is_slice() {
                let object = &sub_exprs[0];
                let start = &sub_exprs[1];
                let end = &sub_exprs[2];
                return write!(f, "{object:?}[{start:?}, {end:?})");
            }

            if self.is_index_non_zero() {
                let object = &sub_exprs[0];
                let index = &sub_exprs[1];
                return if object.ty().is_array() || object.ty().is_slice() {
                    write!(f, "{object:?}[{index:?}]")
                } else if object.ty().is_tuple() {
                    write!(f, "{object:?}.{index:?}")
                } else {
                    assert!(object.ty().is_struct());
                    let i = bigint_to_usize(&index.extract_constant().to_integer());
                    let name = object.ty().struct_def().1[i].0;
                    write!(f, "{object:?}.{name:?}")
                };
            }

            if self.is_index_zero_sized() {
                let object = &sub_exprs[0];
                let ty = &sub_exprs[1];
                return write!(f, "{object:?}<{ty:?}>");
            }

            if self.is_ite() {
                let cond = &sub_exprs[0];
                let true_value = &sub_exprs[1];
                let false_value = &sub_exprs[2];
                return write!(f, "{:?} ? {:?} : {:?}", cond, true_value, false_value);
            }

            if self.is_same_object() {
                let lhs = &sub_exprs[0];
                let rhs = &sub_exprs[1];
                return write!(f, "same_object({lhs:?}, {rhs:?})");
            }

            if self.is_store() {
                let object = &sub_exprs[0];
                let index = &sub_exprs[1];
                let value = &sub_exprs[2];
                return write!(f, "store({object:?}, {index:?}, {value:?})");
            }

            if self.is_offset() {
                let pt = &sub_exprs[0];
                let offset = &sub_exprs[1];
                return write!(f, "{pt:?} + {offset:?}");
            }

            if self.is_pointer_base() {
                let pt = &sub_exprs[0];
                return write!(f, "{pt:?}");
            }

            if self.is_pointer_offset() {
                let pt = &sub_exprs[0];
                return write!(f, "Offset({pt:?})");
            }

            if self.is_pointer_meta() {
                let pt = &sub_exprs[0];
                return write!(f, "Meta({pt:?})");
            }

            if self.is_vec() {
                let pt = &sub_exprs[0];
                let len = &sub_exprs[1];
                let cap = &sub_exprs[2];
                return write!(f, "Vec({pt:?}, {len:?}, {cap:?})");
            }

            if self.is_vec_len() {
                let pt = &sub_exprs[0];
                return write!(f, "VecLen({pt:?})");
            }

            if self.is_vec_cap() {
                let pt = &sub_exprs[0];
                return write!(f, "VecCap({pt:?})");
            }

            if self.is_inner_pointer() {
                let pt = &sub_exprs[0];
                return write!(f, "iptr({pt:?})");
            }

            if self.is_enum() {
                let def = self.ty().enum_def();
                let idx = bigint_to_usize(&sub_exprs[0].extract_constant().to_integer());
                if sub_exprs.len() == 1 {
                    assert!(def.1[idx].1.is_empty());
                    return write!(f, "{:?}", def.1[idx].0);
                } else {
                    return write!(f, "{:?}({:?})", def.1[idx].0, sub_exprs[1]);
                }
            }

            if self.is_as_variant() {
                let def = self.ty().enum_def();
                let x = sub_exprs[0].clone();
                let idx = bigint_to_usize(&sub_exprs[1].extract_constant().to_integer());
                return write!(f, "({x:?} as {:?})", def.1[idx].0);
            }

            if self.is_match_variant() {
                let def = sub_exprs[0].ty().enum_def();
                let x = sub_exprs[0].clone();
                let idx = bigint_to_usize(&sub_exprs[1].extract_constant().to_integer());
                return write!(f, "({x:?} is {:?})", def.1[idx].0);
            }

            if self.is_move() {
                return write!(f, "Move({:?})", sub_exprs[0]);
            }

            if self.is_valid() {
                return write!(f, "Valid({:?})", sub_exprs[0]);
            }

            if self.is_invalid() {
                return write!(f, "Invalid({:?})", sub_exprs[0]);
            }

            if self.is_null_object() {
                return write!(f, "NULL_OBJECT");
            }

            if self.is_unknown() {
                return write!(f, "Unknown");
            }

            println!("Incomplete Debug for Expr");
            Err(Error)
        }
    }
}

pub trait ExprBuilder {
    fn constant_bool(&self, b: bool) -> Expr;
    fn _true(&self) -> Expr;
    fn _false(&self) -> Expr;
    fn constant_integer(&self, i: BigInt, ty: Type) -> Expr;
    fn constant_isize(&self, i: isize) -> Expr;
    fn constant_usize(&self, u: usize) -> Expr;
    fn null(&self, ty: Type) -> Expr;
    fn constant_array(&self, constant: Expr, len: Option<u64>) -> Expr;
    fn constant_struct(&self, fields: Vec<ConstantField>, ty: Type) -> Expr;
    fn mk_symbol(&self, symbol: Symbol, ty: Type) -> Expr;
    fn mk_type(&self, ty: Type) -> Expr;

    fn address_of(&self, object: Expr, ty: Type) -> Expr;
    fn aggregate(&self, operands: Vec<Expr>, ty: Type) -> Expr;

    fn add(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn sub(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn mul(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn div(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn eq(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn ne(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn ge(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn gt(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn le(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn lt(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn and(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn or(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn implies(&self, cond: Expr, conseq: Expr) -> Expr;
    fn not(&self, operand: Expr) -> Expr;
    fn neg(&self, operand: Expr) -> Expr;
    fn ite(&self, cond: Expr, true_value: Expr, false_value: Expr) -> Expr;
    fn cast(&self, operand: Expr, target_ty: Expr) -> Expr;

    fn object(&self, inner_expr: Expr) -> Expr;
    fn slice(&self, object: Expr, start: Expr, len: Expr) -> Expr;
    fn same_object(&self, lhs: Expr, rhs: Expr) -> Expr;
    fn index_non_zero(&self, object: Expr, i: Expr, ty: Type) -> Expr;
    fn index_zero_sized(&self, object: Expr, ty: Type) -> Expr;
    fn store(&self, object: Expr, key: Expr, value: Expr) -> Expr;

    fn offset(&self, pt: Expr, offset: Expr) -> Expr;
    fn pointer_base(&self, pt: Expr) -> Expr;
    fn pointer_offset(&self, pt: Expr) -> Expr;
    fn pointer_meta(&self, pt: Expr) -> Expr;
    fn nonnull(&self, pt: Expr, ty: Type) -> Expr;
    fn unique(&self, pt: Expr, ty: Type) -> Expr;
    fn _box(&self, pt: Expr) -> Expr;
    fn _vec(&self, pt: Expr, len: Expr, cap: Expr, ty: Type) -> Expr;
    fn vec_len(&self, pt: Expr) -> Expr;
    fn vec_cap(&self, pt: Expr) -> Expr;
    fn inner_pointer(&self, pt: Expr) -> Expr;

    fn variant(&self, idx: Expr, data: Option<Expr>, ty: Type) -> Expr;
    fn as_variant(&self, x: Expr, idx: Expr) -> Expr;
    fn match_variant(&self, x: Expr, idx: Expr) -> Expr;

    fn _move(&self, object: Expr) -> Expr;
    fn valid(&self, object: Expr) -> Expr;
    fn invalid(&self, object: Expr) -> Expr;
    fn null_object(&self, ty: Type) -> Expr;
    fn unknown(&self, ty: Type) -> Expr;
}
