
use std::fmt::Error;

use stable_mir::mir::Operand;
use stable_mir::mir::Rvalue;
use stable_mir::mir::Place;
use stable_mir::target::*;
use stable_mir::ty::*;
use stable_mir::CrateDef;

use crate::expr::expr::*;
use crate::expr::constant::*;
use crate::expr::op::*;
use crate::expr::predicates::*;
use crate::expr::ty::*;
use crate::program::program::*;
use crate::symbol::symbol::*;
use crate::symbol::nstring::*;
use super::place_state::*;
use super::projection::*;
use super::symex::*;

impl<'cfg> Symex<'cfg> {
  pub(super) fn make_project(&mut self, place: &Place) -> Expr {
    Projector::new(self).project(place)
  }

  pub(super) fn make_mirconst(&mut self, mirconst: &MirConst) -> Expr {
    match mirconst.kind() {
      ConstantKind::Allocated(allocation) => {
        let ty = Type::from(mirconst.ty());
        let fields =
          if ty.is_struct() { ty.struct_def().1 }
          else { vec![(NString::EMPTY, ty)] };
        let mut value_vec = Vec::new();
        let bytes = &allocation.bytes;
        for i in 0..fields.len() {
          let l = 
            if MachineInfo::target().endian == Endian::Little {
              bytes.len() - allocation.align as usize * (i + 1)
            } else {
              allocation.align as usize * i
            };
          let r = l + allocation.align as usize;
          let mut raw_bytes = Vec::new();
          for j in l..r {
            if let Some(x) = bytes[j] {
              raw_bytes.push(x);
            }
          }
          if fields[i].1.is_bool() {
            assert!(raw_bytes.len() == 1);
            value_vec.push(Constant::Bool(raw_bytes[0] == 1));
            continue;
          }
          let value =
            read_target_integer(
              raw_bytes.as_slice(),
              fields[i].1.is_signed()
            );
          value_vec.push(Constant::Integer(value));
        }

        if ty.is_struct() {
          let mut struct_fields = Vec::new();
          for i in 0..fields.len() {
            struct_fields.push((value_vec[i].clone(), fields[i].1.clone()));
          }
          Ok(self.ctx.constant_struct(struct_fields, ty))
        } else {
          assert!(value_vec.len() == 1);
          if ty.is_bool() {
            Ok(self.ctx.constant_bool(value_vec[0].to_bool()))
          } else if ty.is_integer() {
            let i = value_vec[0].to_integer();
            Ok(self.ctx.constant_integer(i, ty))
          } else {
            Err(Error)
          }
        }
      }
      _ => Err(Error),
    }.expect("Not support")
  }

  pub(super) fn make_operand(&mut self, operand: &Operand) -> Expr {
    match operand {
      Operand::Copy(p) => {
        // TODO: handle copy semantic?
        self.make_project(p)
      },
      Operand::Move(p) => {
        let place = self.make_project(p);
        self.exec_state.update_place_state(place.clone(), PlaceState::Moved);
        place
      },
      Operand::Constant(op) 
        => self.make_mirconst(&op.const_),
    }
  }

  /// Create l1 formula from Rvalue(MIR)
  pub(super) fn make_rvalue(&mut self, rvalue: &Rvalue) -> Expr {
    let ty = self.top().function().rvalue_type(rvalue);
    match rvalue {
      Rvalue::AddressOf(m, p) => {
        let place = self.make_project(p);
        let address_of = self.ctx.address_of(place, ty);
        Ok(address_of)
      },
      Rvalue::Aggregate(k, operands) => {
        // println!("{k:?}\n{:?}", operands.len());
        todo!()
      },
      Rvalue::BinaryOp(mir_op, lop, rop) => {
        let op = BinOp::from(mir_op.clone());
        let lhs = self.make_operand(lop);
        let rhs = self.make_operand(rop);
        let expr =
          match op {
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
        Ok(expr)
      },
      Rvalue::UnaryOp(mir_op, o) => {
        let op = UnOp::from(mir_op.clone());
        let operand = self.make_operand(o);
        let expr =
          match op {
            UnOp::Not => self.ctx.not(operand),
            UnOp::Neg => self.ctx.neg(operand),
          };
        Ok(expr)
      },
      Rvalue::Cast(_, operand, t) => {
        // TODO: handle cast kind
        let op = self.make_operand(operand);
        let target_ty = self.ctx.mk_type(Type::from(t.clone()));
        let cast = self.ctx.cast(op, target_ty);
        Ok(cast)
      },
      Rvalue::Ref(_, _, p) => {
        let object = self.make_project(p);
        // TODO: handle borrow kind.
        let address_of = self.ctx.address_of(object, ty);
        Ok(address_of)
      },
      Rvalue::Use(operand)
        => Ok(self.make_operand(operand)),
      _ => Err(Error),
    }.expect(format!("Do not support: {rvalue:?}").as_str())
  }

  pub(super) fn make_layout(&mut self, arg: &Operand) -> Type {
    match arg {
      Operand::Move(p) => {
        assert!(p.projection.is_empty());
        let mut ty =
          self.exec_state.current_local(p.local, Level::Level2);
        self.rename(&mut ty);
        assert!(ty.is_type());
        Ok(ty.extract_layout())
      },
      Operand::Constant(c) => {
        Ok(Type::from(c.ty()))
      }
      _ => Err(Error),
    }.expect("Do no exits")
  }

  pub(super) fn make_fn_kind(
    &mut self,
    fndef: (FnDef, GenericArgs),
    args: &Vec<Operand>
  ) -> FnKind {
    let name = NString::from(fndef.0.trimmed_name());
    if self.program.contains_function(name) {
      Ok(FnKind::Unwind(self.program.function_idx(name)))
    } else if name == NString::from("Layout::new") {
      assert!(fndef.1.0.len() == 1);
      let ty = fndef.1.0[0].ty().unwrap();
      Ok(FnKind::Layout(Type::from(*ty)))
    } else if name == NString::from("Box::<T>::new") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
      Ok(FnKind::Allocation(AllocKind::Box, ty))
    } else if name == NString::from("alloc") {
      assert!(args.len() == 1);
      let ty = self.make_layout(&args[0]);
      Ok(FnKind::Allocation(AllocKind::Alloc, ty))
    } else if name == NString::from("AsMut::as_mut") {
      Ok(FnKind::AsMut(args[0].clone()))
    } else {
      Err(Error)
    }.expect(format!("Do not support {name:?}").as_str())
  }

  /// Interface for `l2` reaming.
  pub(super) fn rename(&mut self, expr: &mut Expr) {
    self.exec_state.rename(expr, Level::Level2);
  }

  pub(super) fn replace_predicates(&mut self, expr: &mut Expr) {
    match expr.sub_exprs() {
      Some(mut sub_exprs) => {
        let mut has_changed = false;
        for sub_expr in sub_exprs.iter_mut() {
          if sub_expr.has_predicates() {
            has_changed |= true;
            self.replace_predicates(sub_expr);
          }
        }
        if has_changed { expr.replace_sub_exprs(sub_exprs); }
      }
      None => {},
    }

    if expr.is_invalid() {
      let object = expr.extract_object();
      let ptr_indent =
        self
          .ctx
          .pointer_ident(
            if object.ty().is_box() { object.extract_inner_expr() }
            else { 
              self.ctx.address_of(
                object.clone(),
                object.extract_address_type()
              )
            }
          );
      let alloc_array_sym =
        self.exec_state.ns.lookup(NString::ALLOC_SYM);
      let alloc_array =
        self.ctx.object(alloc_array_sym, Ownership::Own);
      let not_alloced =
        self.ctx.not(
          self.ctx.index(
            alloc_array,
            ptr_indent,
            Type::bool_type()
          )
        );
      *expr = not_alloced;
      return;
    }
  }
}