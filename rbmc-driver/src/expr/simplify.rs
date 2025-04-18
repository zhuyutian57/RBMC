use num_bigint::BigInt;

use crate::program::program::bigint_to_usize;

use super::context::*;
use super::expr::*;
use super::op::*;
use super::ty::Type;

impl Expr {
    pub fn simplify(&mut self) {
        if self.is_constant() {
            return;
        }

        if self.ty().is_bool() {
            self.to_nnf(false);
        }

        let sub_exprs = self.simplify_args();
        if sub_exprs == None {
            return;
        }
        let args = sub_exprs.unwrap();

        if self.is_binary() {
            self.simplify_binary(args[0].clone(), args[1].clone());
            return;
        }

        if self.is_unary() {
            self.simplify_unary(args[0].clone());
            return;
        }

        if self.is_ite() {
            self.simplify_ite(args[0].clone(), args[1].clone(), args[2].clone());
            return;
        }

        if self.is_cast() {
            self.simplify_cast(args[0].clone(), self.extract_target_type());
            return;
        }

        if self.is_object() {
            *self = self.ctx.object(args[0].clone());
            return;
        }

        if self.is_same_object() {
            self.simplify_same_object(args[0].clone(), args[1].clone());
            return;
        }

        if self.is_index() {
            self.simplify_index(args[0].clone(), args[1].clone());
            return;
        }

        if self.is_store() {
            self.simplify_store(args[0].clone(), args[1].clone(), args[2].clone());
            return;
        }

        if self.is_vec() {
            let pt = args[0].clone();
            let len = args[1].clone();
            let cap = args[2].clone();
            *self = self.ctx._vec(pt, len, cap, self.ty());
            return;
        }

        if self.is_vec_len() {
            let _vec = args[0].clone();
            *self = if _vec.is_vec() {
                _vec.extract_vec_len()
            } else {
                self.ctx.vec_len(args[0].clone())
            };
            return;
        }

        if self.is_vec_cap() {
            let _vec = args[0].clone();
            *self = if _vec.is_vec() {
                _vec.extract_vec_cap()
            } else {
                self.ctx.vec_cap(args[0].clone())
            };
            return;
        }

        if self.is_inner_pointer() {
            let inner_pt = self.extract_inner_pointer();
            if inner_pt.is_vec() {
                *self = inner_pt.extract_inner_pointer();
            }
            return;
        }
    }

    fn to_nnf(&mut self, is_not: bool) {
        if self.is_binary() {
            if self.extract_bin_op() == BinOp::Implies {
                return;
            }
            let sub_exprs = self.sub_exprs().unwrap();
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

    fn simplify_args(&mut self) -> Option<Vec<Expr>> {
        if let Some(mut sub_exprs) = self.sub_exprs() {
            for sub_expr in sub_exprs.iter_mut() {
                sub_expr.simplify();
            }
            Some(sub_exprs)
        } else {
            None
        }
    }

    fn simplify_binary(&mut self, lhs: Expr, rhs: Expr) {
        match self.extract_bin_op() {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => self.simplify_arith(lhs, rhs),
            BinOp::Eq | BinOp::Ne | BinOp::Ge | BinOp::Gt | BinOp::Le | BinOp::Lt => {
                self.simplify_cmp(lhs, rhs)
            }
            BinOp::And | BinOp::Or | BinOp::Implies => self.simplify_logic(lhs, rhs),
        };
    }

    fn simplify_arith(&mut self, lhs: Expr, rhs: Expr) {
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
        } else {
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

    fn simplify_cmp(&mut self, lhs: Expr, rhs: Expr) {
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
                assert!(lhs.ty().is_any_ptr());
                true
            };
            *self = self.ctx.constant_bool(res);
        } else {
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

    fn simplify_logic(&mut self, lhs: Expr, rhs: Expr) {
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
                    } else {
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
                } else {
                    *self = self.ctx.implies(lhs, rhs);
                }
            }
            _ => todo!("Impossible"),
        };
    }

    fn simplify_unary(&mut self, operand: Expr) {
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

    fn simplify_ite(&mut self, cond: Expr, true_value: Expr, false_value: Expr) {
        if cond.is_true() {
            self.id = true_value.id;
        } else if cond.is_false() {
            self.id = false_value.id;
        } else {
            *self = self.ctx.ite(cond, true_value, false_value);
        }
    }

    fn simplify_cast(&mut self, src: Expr, ty: Type) {
        if src.is_constant() && ty.is_integer() {
            let integer = if src.ty().is_integer() {
                src.extract_constant().to_integer()
            } else {
                assert!(src.ty().is_any_ptr());
                BigInt::ZERO
            };
            *self = self.ctx.constant_integer(integer, ty)
        }
    }

    fn simplify_same_object(&mut self, lhs: Expr, rhs: Expr) {
        if lhs == rhs {
            self.id = Context::TRUE_ID;
        } else {
            *self = self.ctx.same_object(lhs, rhs)
        }
    }

    /// Read-Write simplify
    fn simplify_index(&mut self, object: Expr, i: Expr) {
        let inner_expr = object.extract_inner_expr();
        if inner_expr.is_aggregate() && i.is_constant() {
            let idx = bigint_to_usize(&i.extract_constant().to_integer());
            *self = inner_expr.extract_fields()[idx].clone();
            return;
        } else if inner_expr.is_store() {
            let mut update_index = inner_expr.extract_index();
            let mut update_value = inner_expr.extract_update_value();
            update_index.simplify();
            if i == update_index {
                update_value.simplify();
                *self = update_value;
            }
            return;
        }
        *self = self.ctx.index(object, i, self.ty());
    }

    /// Write-Write simplify
    fn simplify_store(&mut self, object: Expr, i: Expr, value: Expr) {
        let inner_expr = object.extract_inner_expr();
        if inner_expr.is_store() {
            let mut update_index = inner_expr.extract_index();
            update_index.simplify();
            if i == update_index {
                *self = self.ctx.store(object, i, value);
            }
        } else {
            *self = self.ctx.store(object, i, value);
        }
    }
}
