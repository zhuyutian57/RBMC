use num_bigint::BigInt;

use crate::program::program::bigint_to_usize;

use super::context::*;
use super::expr::*;
use super::op::*;

impl Expr {
    pub fn simplify(&mut self) -> bool {
        if self.is_simplified() {
            return false;
        }

        if self.ty().is_bool() {
            self.to_nnf(false);
        }

        let old_id = self.id;

        if self.is_aggregate() {
            self.simplify_aggregate();
        } else if self.is_binary() {
            self.simplify_binary();
        } else if self.is_unary() {
            self.simplify_unary();
        } else if self.is_ite() {
            self.simplify_ite();
        } else if self.is_cast() {
            self.simplify_cast();
        } else if self.is_object() {
            let mut inner_expr = self.extract_inner_expr();
            if inner_expr.simplify() {
                *self = self.ctx.object(inner_expr);
            }
        } else if self.is_same_object() {
            self.simplify_same_object();
        } else if self.is_index() {
            self.simplify_index();
        } else if self.is_store() {
            self.simplify_store();
        } else if self.is_pointer() {
            self.simplify_pointer();
        } else if self.is_pointer_base() {
            self.simplify_pointer_base();
        } else if self.is_pointer_offset() {
            self.simplify_pointer_offset();
        } else if self.is_pointer_meta() {
            self.simplify_pointer_meta();
        } else if self.is_variant() {
            self.simplify_variant();
        } else if self.is_as_variant() {
            self.simplify_as_variant();
        } else if self.is_match_variant() {
            self.simplify_match_variant();
        }

        self.ctx.borrow_mut().simplify_node(self.id);

        self.id != old_id
    }

    fn to_nnf(&mut self, is_not: bool) {
        if self.is_binary() {
            if self.extract_bin_op() == BinOp::Implies {
                return;
            }
            let sub_exprs = self.sub_exprs();
            let mut lhs = sub_exprs[0].clone();
            let mut rhs = sub_exprs[1].clone();
            if lhs.ty().is_bool() {
                lhs.to_nnf(is_not);
            }
            if rhs.ty().is_bool() {
                rhs.to_nnf(is_not);
            }
            if is_not {
                *self = match self.extract_bin_op() {
                    BinOp::Eq => self.ctx.ne(lhs, rhs),
                    BinOp::Ne => self.ctx.eq(lhs, rhs),
                    BinOp::Ge => self.ctx.lt(lhs, rhs),
                    BinOp::Gt => self.ctx.le(lhs, rhs),
                    BinOp::Le => self.ctx.gt(lhs, rhs),
                    BinOp::Lt => self.ctx.ge(lhs, rhs),
                    BinOp::And => self.ctx.or(lhs, rhs),
                    BinOp::Or => self.ctx.and(lhs, rhs),
                    _ => panic!("Impossible"),
                };
            } else {
                *self = match self.extract_bin_op() {
                    BinOp::Eq => self.ctx.eq(lhs, rhs),
                    BinOp::Ne => self.ctx.ne(lhs, rhs),
                    BinOp::Ge => self.ctx.ge(lhs, rhs),
                    BinOp::Gt => self.ctx.gt(lhs, rhs),
                    BinOp::Le => self.ctx.le(lhs, rhs),
                    BinOp::Lt => self.ctx.lt(lhs, rhs),
                    BinOp::And => self.ctx.and(lhs, rhs),
                    BinOp::Or => self.ctx.or(lhs, rhs),
                    _ => panic!("Impossible"),
                };
            }
        } else if self.is_unary() {
            let mut operand = self.extract_inner_expr();
            match self.extract_un_op() {
                UnOp::Not => operand.to_nnf(!is_not),
                _ => panic!("Impossible"),
            };
            *self = operand;
        } else if is_not {
            *self = self.ctx.not(self.clone());
        }
    }

    fn simplifid_args(&self) -> (bool, Vec<Expr>) {
        let mut args = self.sub_exprs();
        let changed = args.iter_mut().fold(false, |acc, arg| acc | arg.simplify());
        (changed, args)
    }

    fn simplify_aggregate(&mut self) {
        let (changed, args) = self.simplifid_args();
        let is_const = args.iter().fold(true, |acc, field| acc && field.is_constant());
        if !changed && !is_const {
            return;
        }
        if self.ty().is_adt() && is_const {
            let fields = args.iter().map(|x| x.extract_constant()).collect::<Vec<_>>();
            *self = self.ctx.constant_adt(fields, self.ty());
        } else if changed {
            *self = self.ctx.aggregate(args, self.ty());
        }
    }

    fn simplify_binary(&mut self) {
        match self.extract_bin_op() {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => self.simplify_arith(),
            BinOp::Eq | BinOp::Ne | BinOp::Ge | BinOp::Gt | BinOp::Le | BinOp::Lt => {
                self.simplify_cmp()
            }
            BinOp::And | BinOp::Or | BinOp::Implies => self.simplify_logic(),
            BinOp::Offset => self.simplify_offset(),
        };
    }

    fn simplify_arith(&mut self) {
        let (changed, args) = self.simplifid_args();
        let lhs = args[0].clone();
        let rhs = args[1].clone();
        if lhs.is_constant() && rhs.is_constant() {
            let a = lhs.extract_constant().to_integer();
            let b = rhs.extract_constant().to_integer();
            let res = match self.extract_bin_op() {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b,
                _ => todo!("Impossible"),
            };
            *self = self.ctx.constant_integer(res, self.ty());
        } else if lhs.is_constant() && lhs.extract_constant().to_integer() == BigInt::ZERO {
            let mut res = match self.extract_bin_op() {
                BinOp::Add => rhs,
                BinOp::Sub => self.ctx.neg(rhs),
                BinOp::Mul | BinOp::Div => self.ctx.constant_integer(BigInt::ZERO, self.ty()),
                _ => todo!("Impossible"),
            };
            res.simplify();
            *self = res;
        } else if rhs.is_constant() && rhs.extract_constant().to_integer() == BigInt::ZERO {
            let mut res = match self.extract_bin_op() {
                BinOp::Add | BinOp::Sub => lhs,
                BinOp::Mul => self.ctx.constant_integer(BigInt::ZERO, self.ty()),
                BinOp::Div => panic!("Div zero"),
                _ => todo!("Impossible"),
            };
            res.simplify();
            *self = res;
        } else if changed {
            // Build with simplified sub-exprs
            *self = match self.extract_bin_op() {
                BinOp::Add => self.ctx.add(lhs, rhs),
                BinOp::Sub => self.ctx.sub(lhs, rhs),
                BinOp::Mul => self.ctx.mul(lhs, rhs),
                BinOp::Div => self.ctx.div(lhs, rhs),
                _ => todo!("Impossible"),
            };
        }
    }

    fn simplify_cmp(&mut self) {
        let (changed, args) = self.simplifid_args();
        let lhs = args[0].clone();
        let rhs = args[1].clone();
        if lhs.is_constant() && rhs.is_constant() {
            let res = if lhs.ty().is_integer() {
                let a = lhs.extract_constant().to_integer();
                let b = rhs.extract_constant().to_integer();
                match self.extract_bin_op() {
                    BinOp::Eq => a == b,
                    BinOp::Ne => a != b,
                    BinOp::Ge => a >= b,
                    BinOp::Gt => a > b,
                    BinOp::Le => a <= b,
                    BinOp::Lt => a < b,
                    _ => todo!("Impossible"),
                }
            } else if lhs.ty().is_bool() {
                let a = lhs.extract_constant().to_bool();
                let b = rhs.extract_constant().to_bool();
                match self.extract_bin_op() {
                    BinOp::Eq => a == b,
                    BinOp::Ne => a != b,
                    BinOp::Ge => a >= b,
                    BinOp::Gt => a > b,
                    BinOp::Le => a <= b,
                    BinOp::Lt => a < b,
                    _ => todo!("Impossible"),
                }
            } else {
                assert!(lhs.ty().is_primitive_ptr());
                true
            };
            *self = self.ctx.constant_bool(res);
        } else if self.extract_bin_op() == BinOp::Eq && lhs == rhs {
            // Special case
            *self = self.ctx._true();
        } else if changed {
            *self = match self.extract_bin_op() {
                BinOp::Eq => self.ctx.eq(lhs, rhs),
                BinOp::Ne => self.ctx.ne(lhs, rhs),
                BinOp::Ge => self.ctx.ge(lhs, rhs),
                BinOp::Gt => self.ctx.gt(lhs, rhs),
                BinOp::Le => self.ctx.le(lhs, rhs),
                BinOp::Lt => self.ctx.lt(lhs, rhs),
                _ => todo!("Impossible"),
            };
        }
    }

    fn simplify_logic(&mut self) {
        let (changed, args) = self.simplifid_args();
        let lhs = args[0].clone();
        let rhs = args[1].clone();
        match self.extract_bin_op() {
            BinOp::And => {
                if lhs.is_true() {
                    self.id = rhs.id;
                } else if rhs.is_true() {
                    self.id = lhs.id;
                } else if lhs.is_false() || rhs.is_false() {
                    self.id = Context::FALSE_ID;
                } else if lhs == rhs {
                    self.id = lhs.id;
                } else {
                    let mut not_rhs = self.ctx.not(rhs.clone());
                    not_rhs.simplify();
                    if lhs == not_rhs {
                        *self = self.ctx._false();
                    } else {
                        *self = self.ctx.and(lhs, rhs);
                    }
                }
            }
            BinOp::Or => {
                if lhs.is_false() {
                    self.id = rhs.id;
                } else if rhs.is_false() {
                    self.id = lhs.id;
                } else if lhs.is_true() || rhs.is_true() {
                    self.id = Context::TRUE_ID;
                } else if lhs == rhs {
                    self.id = lhs.id;
                } else {
                    let mut not_rhs = self.ctx.not(rhs.clone());
                    not_rhs.simplify();
                    if lhs == not_rhs {
                        *self = self.ctx._true();
                    } else if changed {
                        *self = self.ctx.or(lhs, rhs);
                    }
                }
            }
            BinOp::Implies => {
                if lhs.is_false() || rhs.is_true() {
                    self.id = Context::TRUE_ID;
                } else if lhs.is_true() && rhs.is_false() {
                    self.id = Context::FALSE_ID;
                } else if lhs == rhs {
                    self.id = Context::TRUE_ID;
                } else if changed {
                    *self = self.ctx.implies(lhs, rhs);
                }
            }
            _ => todo!("Impossible"),
        };
    }

    fn simplify_offset(&mut self) {
        let (changed, args) = self.simplifid_args();
        let lhs = args[0].clone();
        let rhs = args[1].clone();
        if rhs.is_constant() && rhs.extract_constant().to_integer() == BigInt::ZERO {
            *self = lhs;
        } else if changed {
            *self = self.ctx.offset(lhs, rhs);
        }
    }

    fn simplify_unary(&mut self) {
        let (_, args) = self.simplifid_args();
        let operand = args[0].clone();
        match self.extract_un_op() {
            UnOp::Not | UnOp::Neg => {
                if operand.is_unary() && operand.extract_un_op() == self.extract_un_op() {
                    self.id = operand.extract_inner_expr().id;
                } else if operand.is_true() {
                    self.id = Context::FALSE_ID;
                } else if operand.is_false() {
                    self.id = Context::TRUE_ID;
                }
            }
            _ => todo!("Not support"),
        }
    }

    fn simplify_ite(&mut self) {
        let (changed, args) = self.simplifid_args();
        let cond = args[0].clone();
        let true_value = args[1].clone();
        let false_value = args[2].clone();
        if cond.is_true() {
            self.id = true_value.id;
        } else if cond.is_false() {
            self.id = false_value.id;
        } else if true_value == false_value {
            self.id = true_value.id;
        } else if changed {
            *self = self.ctx.ite(cond, true_value, false_value);
        }
    }

    fn simplify_cast(&mut self) {
        let (changed, args) = self.simplifid_args();
        let src = args[0].clone();
        let ty = args[1].clone().extract_type();
        if src.is_constant() {
            if src.ty().is_integer() && ty.is_integer() {
                let i = src.extract_constant().to_integer();
                *self = self.ctx.constant_integer(i, ty);
            } else {
                assert!(src.is_null() && ty.is_usize());
                *self = self.ctx.constant_usize(0);
            }
        } else if changed {
            *self = self.ctx.cast(src, self.ctx.mk_type(ty));
        }
    }

    fn simplify_same_object(&mut self) {
        let (changed, args) = self.simplifid_args();
        let lhs = args[0].clone();
        let rhs = args[1].clone();
        if lhs == rhs {
            *self = self.ctx._true();
        } else if changed {
            *self = self.ctx.same_object(lhs, rhs);
        }
    }

    /// Read-Write simplify
    fn simplify_index(&mut self) {
        let (changed, args) = self.simplifid_args();
        let object = args[0].clone();
        let i = args[1].clone();
        let inner_expr = object.extract_inner_expr();
        if i.is_constant() {
            let idx = bigint_to_usize(&i.extract_constant().to_integer());
            if inner_expr.is_aggregate() {
                *self = inner_expr.extract_fields()[idx].clone();
            } else if inner_expr.is_constant() {
                if inner_expr.ty().is_struct() || inner_expr.ty().is_tuple() {
                    let (fields, _) = inner_expr.extract_constant().to_adt();
                    let ty = inner_expr.ty().field_type_exclude_zst(idx);
                    *self = self.ctx.constant(fields[idx].clone(), ty);
                } else {
                    assert!(inner_expr.ty().is_enum());
                    let data = &inner_expr.extract_constant().to_adt().0[1];
                    let (fields, _) = data.to_adt();
                    *self = self.ctx.constant(fields[idx].clone(), self.ty());
                }
            }
        } else if inner_expr.is_store() {
            let mut update_index = inner_expr.extract_index();
            let mut update_value = inner_expr.extract_update_value();
            update_index.simplify();
            if i == update_index {
                update_value.simplify();
                *self = update_value;
            }
        } else if changed {
            *self = self.ctx.index(object, i, self.ty());
        }
    }

    /// Write-Write simplify
    fn simplify_store(&mut self) {
        let (changed, args) = self.simplifid_args();
        let object = args[0].clone();
        let i = args[1].clone();
        let value = args[2].clone();
        let inner_expr = object.extract_inner_expr();
        if inner_expr.is_store() {
            let mut update_index = inner_expr.extract_index();
            update_index.simplify();
            if i == update_index {
                *self = self.ctx.store(object, i, value);
            }
        } else if changed {
            *self = self.ctx.store(object, i, value);
        }
    }

    fn simplify_pointer(&mut self) {
        let (changed, args) = self.simplifid_args();
        let address = args[0].clone();
        let meta = args[1].clone();
        if address.is_null() {
            *self = address;
        } else if changed {
            *self = self.ctx.pointer(address, Some(meta), self.ty());
        }
    }

    fn simplify_pointer_base(&mut self) {
        let (changed, args) = self.simplifid_args();
        let expr = args[0].clone();
        if expr.is_pointer() {
            let mut base = self.ctx.pointer_base(expr.extract_inner_pointer());
            base.simplify();
            *self = base;
        } else if expr.is_offset() {
            let mut base = self.ctx.pointer_base(expr.extract_lhs());
            base.simplify();
            *self = base;
        } else if changed {
            *self = self.ctx.pointer_base(expr);
        }
    }

    fn simplify_pointer_offset(&mut self) {
        let (changed, args) = self.simplifid_args();
        let expr = args[0].clone();
        if expr.is_address_of() {
            *self = self.ctx.constant_usize(0);
        } else if expr.is_pointer() {
            let mut offset = self.ctx.pointer_offset(expr.extract_inner_pointer());
            offset.simplify();
            *self = offset;
        } else if expr.is_offset() {
            let l_offset = self.ctx.pointer_offset(expr.extract_lhs());
            let r_offset = expr.extract_rhs();
            let mut offset = self.ctx.add(l_offset, r_offset);
            offset.simplify();
            *self = offset;
        } else if changed {
            *self = self.ctx.pointer_offset(expr);
        }
    }

    fn simplify_pointer_meta(&mut self) {
        let (changed, args) = self.simplifid_args();
        let expr = args[0].clone();
        if expr.is_pointer() {
            *self = expr.extract_pointer_meta();
        } else if changed {
            *self = self.ctx.pointer_meta(expr);
        }
    }

    fn simplify_variant(&mut self) {
        let (changed, args) = self.simplifid_args();
        let i = args[0].clone();
        let data = args[1].clone();
        if data.is_constant() {
            let i = i.extract_constant();
            let d = data.extract_constant();
            *self = self.ctx.constant_adt(vec![i, d], self.ty());
        } else if changed {
            *self = self.ctx.variant(i, data, self.ty());
        }
    }

    fn simplify_as_variant(&mut self) {
        let (changed, args) = self.simplifid_args();
        let _enum = args[0].clone();
        let i = args[1].clone();
        let j = bigint_to_usize(&i.extract_constant().to_integer());
        if _enum.is_variant() {
            let idx = _enum.extract_variant_idx();
            assert!(j == idx);
            let data = _enum.extract_variant_data();
            if data.is_constant() {
                *self = self
                    .ctx
                    .constant_adt(vec![i.extract_constant(), data.extract_constant()], self.ty());
                return;
            }
        } else if _enum.is_constant() {
            let b = _enum.extract_constant().to_adt().0[0].to_integer();
            let i = bigint_to_usize(&b);
            assert!(i == j);
            *self = _enum.clone();
            return;
        }
        if changed {
            *self = self.ctx.as_variant(_enum, i);
        }
    }

    fn simplify_match_variant(&mut self) {
        let (changed, args) = self.simplifid_args();
        let _enum = args[0].clone();
        let i = args[1].clone();
        let j = bigint_to_usize(&i.extract_constant().to_integer());
        let idx = if _enum.is_constant() {
            Some(_enum.extract_constant().to_adt().0[0].to_integer())
        } else if _enum.is_variant() || _enum.is_as_variant() {
            Some(_enum.extract_variant_idx().into())
        } else {
            None
        };
        if let Some(b) = idx {
            let variant_idx = bigint_to_usize(&b);
            *self = if j == variant_idx { self.ctx._true() } else { self.ctx._false() };
        } else if changed {
            *self = self.ctx.match_variant(_enum, i);
        }
    }
}
