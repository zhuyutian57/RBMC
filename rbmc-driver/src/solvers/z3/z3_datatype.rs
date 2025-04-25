use z3::DatatypeAccessor;
use z3::ast::*;

use super::z3_conv::*;
use crate::expr::expr::*;
use crate::expr::ty::*;
use crate::solvers::smt::smt_conv::*;
use crate::solvers::smt::smt_datatype::*;
use crate::symbol::nstring::NString;

impl<'ctx> DataType<z3::Sort<'ctx>, z3::ast::Dynamic<'ctx>> for Z3Conv<'ctx> {
    fn create_datatype_sign(&mut self, ty: Type) -> DataTypeSign {
        let mut sign = (NString::EMPTY, Vec::new());
        if ty.is_struct() {
            let def = ty.struct_def();
            sign.0 = NString::from("_struct_") + def.0;
            for fdef in def.1.iter() {
                if fdef.1.is_zero_sized_type() {
                    continue;
                }
                sign.1.push(fdef.1);
            }
        } else if ty.is_tuple() {
            sign.0 = ty.name();
        } else {
            assert!(ty.is_enum());
            let def = ty.enum_def();
            sign.0 = NString::from("_enum_") + def.0;
            // Flattern all variants
            for vdef in def.1.iter() {
                for fdef in vdef.1.iter() {
                    if fdef.1.is_zero_sized_type() {
                        continue;
                    }
                    sign.1.push(fdef.1);
                }
            }
        }
        sign
    }

    fn create_datatype(&mut self, sign: DataTypeSign, variants: Variants) -> z3::Sort<'ctx> {
        let mut builder = z3::DatatypeBuilder::new(&self.z3_ctx, sign.0.to_string());
        for variant in &variants {
            let mut fields = Vec::new();
            for (name, ty) in variant.1.iter() {
                let accessor = DatatypeAccessor::Sort(self.convert_sort(*ty));
                fields.push((name.as_str(), accessor));
            }
            builder = builder.variant(variant.0.as_str(), fields);
        }
        let dtsort = builder.finish();
        let sort = dtsort.sort.clone();
        self.datatypes.insert(sign, dtsort);
        sort
    }

    fn mk_struct_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
        assert!(ty.is_struct());
        let def = ty.struct_def();
        let sign = self.create_datatype_sign(ty);

        if self.datatypes.contains_key(&sign) {
            return self.datatypes.get(&sign).unwrap().sort.clone();
        }

        let mut fields = Vec::new();
        for (field, ty) in def.1.iter() {
            if ty.is_zero_sized_type() {
                continue;
            }
            let field_name = sign.0 + "_" + *field;
            fields.push((field_name, *ty));
        }
        let variants = vec![(sign.0, fields)];

        self.create_datatype(sign, variants)
    }

    fn mk_tuple_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
        assert!(ty.is_tuple() && !ty.is_unit());
        let def = ty.tuple_def();
        let sign = self.create_datatype_sign(ty);

        if self.datatypes.contains_key(&sign) {
            return self.datatypes.get(&sign).unwrap().sort.clone();
        }

        let mut fields = Vec::new();
        for (i, ty) in def.iter().enumerate() {
            if ty.is_zero_sized_type() {
                continue;
            }
            let field_name = sign.0 + "_" + i.to_string();
            fields.push((field_name, *ty));
        }
        let variants = vec![(sign.0, fields)];

        self.create_datatype(sign, variants)
    }

    fn mk_enum_sort(&mut self, ty: Type) -> z3::Sort<'ctx> {
        assert!(ty.is_enum());
        let def = ty.enum_def();
        let sign = self.create_datatype_sign(ty);

        if self.datatypes.contains_key(&sign) {
            return self.datatypes.get(&sign).unwrap().sort.clone();
        }

        let mut variants = Vec::new();
        for vdef in def.1.iter() {
            let vname = vdef.0;
            let mut fields = Vec::new();
            for fdef in vdef.1.iter() {
                if ty.is_zero_sized_type() {
                    continue;
                }
                let fname = fdef.0;
                fields.push((fname, fdef.1));
            }
            variants.push((vname, fields));
        }

        self.create_datatype(sign, variants)
    }

    fn mk_struct(
        &mut self,
        fields: &Vec<z3::ast::Dynamic<'ctx>>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        assert!(ty.is_struct());
        let sign = self.create_datatype_sign(ty);
        if !self.datatypes.contains_key(&sign) {
            self.mk_struct_sort(ty);
        }
        let dtsort = self.datatypes.get(&sign).unwrap();
        let f = &dtsort.variants[0].constructor;
        let mut args = Vec::new();
        for arg in fields.iter() {
            args.push(arg as &dyn Ast<'_>);
        }
        f.apply(args.as_slice())
    }

    fn mk_tuple(
        &mut self,
        fields: &Vec<z3::ast::Dynamic<'ctx>>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        assert!(ty.is_tuple());
        let sign = self.create_datatype_sign(ty);
        if !self.datatypes.contains_key(&sign) {
            self.mk_tuple_sort(ty);
        }
        let dtsort = self.datatypes.get(&sign).unwrap();
        let f = &dtsort.variants[0].constructor;
        let mut args = Vec::new();
        for arg in fields.iter() {
            args.push(arg as &dyn Ast<'_>);
        }
        f.apply(args.as_slice())
    }

    fn mk_tuple_select(
        &mut self,
        object: z3::ast::Dynamic<'ctx>,
        field: usize,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        assert!(ty.is_struct() || ty.is_tuple());
        let sign = self.create_datatype_sign(ty);
        if !self.datatypes.contains_key(&sign) {
            if ty.is_struct() {
                self.mk_struct_sort(ty);
            } else {
                self.mk_tuple_sort(ty);
            }
        }
        let mut i = field;
        ty.fix_index_field(&mut i);
        let args = &[&object as &dyn Ast];
        self.datatypes.get(&sign).unwrap().variants[0].accessors[i].apply(args)
    }

    fn mk_tuple_store(
        &mut self,
        object: z3::ast::Dynamic<'ctx>,
        field: usize,
        value: z3::ast::Dynamic<'ctx>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        assert!(ty.is_struct() || ty.is_tuple());
        let sign = self.create_datatype_sign(ty);
        if !self.datatypes.contains_key(&sign) {
            if ty.is_struct() {
                self.mk_struct_sort(ty);
            } else {
                self.mk_tuple_sort(ty);
            }
        }
        let n = self.datatypes.get(&sign).unwrap().variants[0].accessors.len();
        let mut j = field;
        ty.fix_index_field(&mut j);
        let mut fields_values = Vec::with_capacity(n);
        for i in 0..n {
            if j != i {
                fields_values.push(self.mk_tuple_select(object.clone(), i, ty));
            } else {
                fields_values.push(value.clone());
            }
        }
        let args = fields_values.iter().map(|x| x as &dyn Ast).collect::<Vec<_>>();
        self.datatypes.get(&sign).unwrap().variants[0].constructor.apply(&args.as_slice())
    }

    fn mk_variant(
        &mut self,
        idx: usize,
        data: Option<z3::ast::Dynamic<'ctx>>,
        ty: Type,
    ) -> z3::ast::Dynamic<'ctx> {
        let sign = self.create_datatype_sign(ty);
        if !self.datatypes.contains_key(&sign) {
            self.mk_enum_sort(ty);
        }
        let dtsort = self.datatypes.get(&sign).unwrap();
        let f = &dtsort.variants[idx].constructor;
        let mut args = Vec::new();
        if let Some(x) = &data {
            args.push(x as &dyn Ast<'_>);
        }
        f.apply(args.as_slice())
    }

    fn mk_as_variant(&mut self, _enum: Expr, idx: usize) -> z3::ast::Dynamic<'ctx> {
        let ty: Type = _enum.ty();
        let sign = self.create_datatype_sign(ty);
        if !self.datatypes.contains_key(&sign) {
            self.mk_enum_sort(ty);
        }
        let args = [&self.convert_ast(_enum) as &dyn Ast];
        self.datatypes.get(&sign).unwrap().variants[idx].accessors[0].apply(&args)
    }

    fn mk_match_variant(&mut self, _enum: Expr, idx: usize) -> z3::ast::Dynamic<'ctx> {
        let ty = _enum.ty();
        let sign = self.create_datatype_sign(ty);
        if !self.datatypes.contains_key(&sign) {
            self.mk_enum_sort(ty);
        }
        let x = self.convert_ast(_enum);
        let dtsort = self.datatypes.get(&sign).unwrap();
        let f = &dtsort.variants[idx].tester;
        f.apply(&[&x as &dyn Ast])
    }
}
