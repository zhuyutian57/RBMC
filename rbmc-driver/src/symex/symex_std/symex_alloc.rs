use stable_mir::CrateDef;
use stable_mir::mir::mono::Instance;

use super::super::symex::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::symbol::nstring::*;
use crate::symex::place_state::PlaceState;
use crate::symex::projection::Mode;

/// This mod defines symbolic execution of api in std::alloc

impl<'cfg> Symex<'cfg> {
    pub fn symex_alloc_api(&mut self, instance: Instance, args: Vec<Expr>, dest: Expr) {
        let fty = Type::from(instance.ty());
        let name = NString::from(fty.fn_def().0.trimmed_name());
        if name == "alloc" {
            self.symex_alloc(dest, args);
        } else if name == "dealloc" {
            self.symex_dealloc(args);
        } else if name.starts_with("Layout".into()) {
            self.symex_layout_api(instance, args, dest);
        } else {
            panic!("Not support {name:?}");
        }
    }

    fn symex_alloc(&mut self, dest: Expr, args: Vec<Expr>) {
        let mut layout = args[0].clone();
        self.replace_predicates(&mut layout);
        self.rename(&mut layout);
        assert!(layout.is_type());
        let object = self.exec_state.new_object(layout.extract_type());

        let lhs = dest.clone();
        let address_of = self.ctx.address_of(object.clone(), lhs.ty());

        self.assign(lhs, address_of, self.ctx._true().into());

        self.track_new_object(object.clone());

        let place_state = PlaceState::Alive;
        self.exec_state.update_place_state(object, place_state);
    }

    fn symex_dealloc(&mut self, args: Vec<Expr>) {
        let pt = args[0].clone();
        let mut layout = args[1].clone();
        self.replace_predicates(&mut layout);
        self.rename(&mut layout);
        let ty = layout.extract_type();
        assert!(pt.ty().is_ptr());
        // Generate assertions
        self.make_deref(pt.clone(), Mode::Dealloc, self.ctx._true().into(), ty);

        self.top_mut().cur_state.dealloc_objects(pt.clone());
        self.top_mut().cur_state.remove_pointer(pt.clone());

        let alloc_array = self.exec_state.ns.lookup_object(NString::ALLOC_SYM);
        let index = self.ctx.index(alloc_array, pt, Type::bool_type());
        self.assign(index, self.ctx._false(), self.ctx._true().into());
    }

    fn symex_layout_api(&mut self, instance: Instance, args: Vec<Expr>, dest: Expr) {
        let name = NString::from(instance.trimmed_name());
        if name.starts_with("Layout::new".into())
            || name.starts_with("Layout::for_value_raw".into())
        {
            let ty = Type::from(instance.args().0[0].expect_ty());
            self.symex_assign_layout(dest, ty);
        } else if name == "Layout::size" || name == "Layout::align" {
            let pt = args[0].clone();
            let mut ty_expr = self.make_deref(
                pt.clone(),
                Mode::Read,
                self.ctx._true().into(),
                pt.ty().pointee_ty(),
            );
            self.rename(&mut ty_expr);
            assert!(ty_expr.is_type());
            if name == "Layout::size" {
                self.symex_layout_size(dest, ty_expr.extract_type());
            } else {
                self.symex_layout_align(dest, ty_expr.extract_type());
            }
        } else {
            todo!("{name:?}");
        }
    }

    fn symex_assign_layout(&mut self, dest: Expr, ty: Type) {
        self.assign(dest, self.ctx.mk_type(ty), self.ctx._true().into());
    }

    fn symex_layout_size(&mut self, dest: Expr, ty: Type) {
        let size = self.ctx.constant_usize(ty.size());
        self.assign(dest, size, self.ctx._true().into());
    }

    fn symex_layout_align(&mut self, dest: Expr, ty: Type) {
        let align = self.ctx.constant_usize(ty.align());
        self.assign(dest, align, self.ctx._true().into());
    }
}
