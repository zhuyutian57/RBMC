use std::fmt::Debug;

use stable_mir::mir::mono::Instance;
use stable_mir::CrateDef;
use stable_mir::mir::*;
use stable_mir::ty::*;

use crate::symbol::nstring::NString;

pub type Variant = Vec<FieldDef>;
pub type Variants = Vec<(NString, Variant)>;

pub type EnumDef = (NString, Variants);
pub type FieldDef = (NString, Type);
pub type StructDef = (NString, Vec<FieldDef>);
pub type TupleDef = Vec<Type>;
pub type FunctionDef = (FnDef, GenericArgs);

/// We handle some functions in `std` as builtin functions in symex. That means we
/// execute them by their semantic instead unwinding the body. Because we focus on
/// the memory safety issues on user program.
/// 
/// `Layout::*`: All types in our memory models are field-level. We do not handle size
/// and alignment now.
/// 
/// `std::alloc::alloc`: The `alloc` function is a wrapper of `__rust_alloc`. In our
/// memory model, we assume all allocations are successful. No need to unwind its body.
/// `dealloc` is handled similiarly.
/// 
/// `Box::*`: `Box` is a special struct in rust. In our memory model, `Box<T>` is a
/// primitive type. Thus, some functions are executed directly instead of unwinding.
const RUST_BUILTIN_FUNCTIONS: &[&str] = &[
    "alloc",
    "dealloc",
    // Box
    "Box::<T>::new",
    // Layout
    "Layout::new",
    "Layout::for_value_raw",
    "Layout::size",
    "Layout::align",
    // Slice
    "slice_index_order_fail",
    "slice_start_index_len_fail",
    "slice_end_index_len_fail",
    // Panic
    "panic_nounwind",
];

/// A wrapper for `Ty` in MIR
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Type(pub Ty);

impl Type {
    pub fn unit_type() -> Self {
        Type::from(Ty::new_tuple(&[]))
    }

    pub fn bool_type() -> Self {
        Type::from(Ty::bool_ty())
    }

    pub fn signed_type(ty: IntTy) -> Self {
        Type::from(Ty::signed_ty(ty))
    }

    pub fn isize_type() -> Self {
        Type::signed_type(IntTy::Isize)
    }

    pub fn unsigned_type(ty: UintTy) -> Self {
        Type::from(Ty::unsigned_ty(ty))
    }

    pub fn usize_type() -> Self {
        Type::unsigned_type(UintTy::Usize)
    }

    pub fn array_type(elem_ty: Type, len: u64) -> Self {
        Type::from(
            Ty::try_new_array(elem_ty.0, len)
                .expect(format!("({elem_ty:?}, {len}) is wrong for an array type").as_str()),
        )
    }

    pub fn infinite_array_type(elem_ty: Type) -> Self {
        // Array with len 0 as const array type
        Type::array_type(elem_ty, 0)
    }

    pub fn slice_type(elem_ty: Type) -> Self {
        Type::from(Ty::from_rigid_kind(RigidTy::Slice(elem_ty.0)))
    }

    pub fn slice_type_from_array_type(array_type: Type) -> Self {
        assert!(array_type.is_array());
        Type::slice_type(array_type.elem_type())
    }

    pub fn ptr_type(pointee_type: Type, m: Mutability) -> Self {
        Type::from(Ty::new_ptr(pointee_type.0, m))
    }

    pub fn box_type(inner_type: Type) -> Self {
        Ty::new_box(inner_type.0).into()
    }

    pub fn inner_pointer_type(&self) -> Self {
        assert!(self.is_smart_ptr());
        Type::ptr_type(self.pointee_ty(), Mutability::Not)
    }

    pub fn tuple_type(sub_types: Vec<Type>) -> Self {
        let stypes = sub_types.iter().map(|x| x.0).collect::<Vec<_>>();
        Ty::new_tuple(&stypes).into()
    }

    pub fn is_unit(&self) -> bool {
        self.0.kind().is_unit()
    }

    pub fn is_bool(&self) -> bool {
        self.0.kind().is_bool()
    }

    pub fn is_signed(&self) -> bool {
        self.0.kind().is_signed()
    }

    pub fn is_isize(&self) -> bool {
        *self == Type::isize_type()
    }

    pub fn is_unsigned(&self) -> bool {
        self.0.kind().is_integral() && !self.is_signed()
    }

    pub fn is_usize(&self) -> bool {
        *self == Type::usize_type()
    }

    pub fn is_integer(&self) -> bool {
        self.0.kind().is_integral()
    }

    pub fn is_primitive(&self) -> bool {
        self.0.kind().is_primitive()
    }

    pub fn is_enum(&self) -> bool {
        self.0.kind().is_enum()
    }

    pub fn is_array(&self) -> bool {
        self.0.kind().is_array()
    }

    pub fn is_slice(&self) -> bool {
        self.0.kind().is_slice()
    }

    pub fn is_fn(&self) -> bool {
        self.0.kind().is_fn()
    }

    pub fn is_layout(&self) -> bool {
        self.name() == "Layout"
    }

    pub fn is_struct(&self) -> bool {
        self.0.kind().is_struct() && !self.is_layout()
    }

    pub fn is_empty_struct(&self) -> bool {
        self.is_struct() && self.struct_def().1.is_empty()
    }

    pub fn is_tuple(&self) -> bool {
        match self.0.kind().rigid() {
            Some(r) => matches!(r, RigidTy::Tuple(..)),
            None => false,
        }
    }

    pub fn is_ref(&self) -> bool {
        self.0.kind().is_ref()
    }

    pub fn is_ptr(&self) -> bool {
        self.0.kind().is_raw_ptr()
    }

    pub fn is_const_ptr(&self) -> bool {
        match self.0.kind() {
            TyKind::RigidTy(RigidTy::RawPtr(_, m)) => m == Mutability::Not,
            _ => false,
        }
    }

    pub fn is_slice_ptr(&self) -> bool {
        self.is_primitive_ptr() && self.pointee_ty().is_slice()
    }

    pub fn is_nonnull(&self) -> bool {
        self.name() == "NonNull"
    }

    pub fn is_unique(&self) -> bool {
        self.name() == "Unique"
    }

    pub fn is_box(&self) -> bool {
        self.0.kind().is_box()
    }

    pub fn is_vec(&self) -> bool {
        self.name() == "Vec"
    }

    pub fn is_primitive_ptr(&self) -> bool {
        self.is_ptr() || self.is_ref()
    }

    pub fn is_smart_ptr(&self) -> bool {
        self.is_box() || self.is_vec()
    }

    pub fn is_any_ptr(&self) -> bool {
        self.is_primitive_ptr() || self.is_smart_ptr()
    }

    pub fn is_rbmc_nondet(&self) -> bool {
        if !self.is_fn() { return false; }
        return self.fn_def().0.name() == "rbmc::nondet::<T>";
    }

    pub fn is_rust_builtin_function(&self) -> bool {
        if !self.is_fn() { return false; }
        let name = self.fn_def().0.trimmed_name();
        return RUST_BUILTIN_FUNCTIONS.contains(&name.as_str());
    }

    pub fn is_builtin_function(&self) -> bool {
        self.is_rbmc_nondet() || self.is_rust_builtin_function()
    }

    pub fn is_zero_sized_type(&self) -> bool {
        self.is_unit() || self.is_empty_struct()
    }

    pub fn pointee_ty(&self) -> Self {
        assert!(self.is_any_ptr());
        match self.0.kind() {
            TyKind::RigidTy(r) => match r {
                RigidTy::Adt(def, args) => {
                    let elem_ty = match &args.0[0] {
                        GenericArgKind::Type(ty) => Type::from(ty.clone()),
                        _ => panic!(),
                    };
                    if self.is_box() { elem_ty } else { Type::infinite_array_type(elem_ty) }
                }
                RigidTy::RawPtr(ty, ..) | RigidTy::Ref(_, ty, ..) => Type::from(ty),
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    pub fn array_len(&self) -> Option<usize> {
        assert!(self.is_array());
        let size = match self.0.kind() {
            TyKind::RigidTy(r) => match r {
                RigidTy::Array(_, c) => c.eval_target_usize(),
                _ => panic!("Not array"),
            },
            _ => panic!("Not array"),
        }
        .expect("Wrong array size");
        if size == 0 { None } else { Some(size as usize) }
    }

    /// Assume that all index is integer.
    pub fn array_domain(&self) -> Type {
        assert!(self.is_array());
        Type::unsigned_type(UintTy::Usize)
    }

    /// Range for array/slice
    pub fn elem_type(&self) -> Type {
        assert!(self.is_array() || self.is_slice());
        if let TyKind::RigidTy(r) = self.0.kind() {
            return match r {
                RigidTy::Array(t, _) | RigidTy::Slice(t) => Type::from(t),
                _ => panic!("Impossible"),
            };
        }
        panic!("Impossible")
    }

    pub fn enum_def(&self) -> EnumDef {
        assert!(self.is_enum());
        let mut def = (self.name(), Vec::new());
        if let TyKind::RigidTy(r) = self.0.kind() {
            if let RigidTy::Adt(adt, args) = r {
                def.0 = NString::from(adt.trimmed_name());
                for variant in adt.variants() {
                    let variant_name = NString::from(variant.name());
                    // Type of a variant is a tuple
                    let mut ftypes = Vec::new();
                    for fdef in variant.fields() {
                        ftypes.push(Type(fdef.ty_with_args(&args)));
                    }
                    let tuple_type = Type::tuple_type(ftypes);
                    let fname =  if !tuple_type.is_unit() {
                        NString::from("_data_") + variant_name
                    } else {
                        NString::from("-")
                    };
                    def.1.push((variant_name, vec![(fname, tuple_type)]));
                }
            }
        }
        def
    }

    pub fn enum_variant_data_type(&self, variant_idx: usize) -> Self {
        assert!(self.is_enum());
        let def = self.enum_def();
        assert!(variant_idx < def.1.len());
        def.1[variant_idx].1[0].1
    }

    pub fn fields(&self) -> usize {
        assert!(self.is_struct() || self.is_tuple());
        match self.0.kind() {
            TyKind::RigidTy(RigidTy::Adt(adt, _)) => adt.variants()[0].fields().len(),
            TyKind::RigidTy(RigidTy::Tuple(def)) => def.len(),
            _ => panic!("Impossible"),
        }
    }

    pub fn struct_def(&self) -> StructDef {
        assert!(self.is_struct());
        let mut def = (self.name(), Vec::new());
        if let TyKind::RigidTy(r) = self.0.kind() {
            if let RigidTy::Adt(adt, args) = r {
                for field in adt.variants()[0].fields() {
                    let fty = field.ty_with_args(&args);
                    def.1.push((NString::from(field.name.clone()), Type::from(fty)));
                }
            }
        }
        def
    }

    pub fn tuple_def(&self) -> TupleDef {
        match self.0.kind().rigid() {
            Some(r) => match r {
                RigidTy::Tuple(fields) => fields.iter().map(|t| Type::from(t)).collect::<Vec<_>>(),
                _ => panic!("Not tuple"),
            },
            None => panic!("Not tuple"),
        }
    }

    pub fn field_type(&self, field: usize) -> Type {
        if self.is_struct() {
            let sdef = self.struct_def();
            assert!(field < sdef.1.len());
            sdef.1[field].1
        } else if self.is_tuple() {
            let tdef = self.tuple_def();
            assert!(field < tdef.len());
            tdef[field]
        } else {
            panic!("Not struct and tuple")
        }
    }

    pub fn fn_def(&self) -> FunctionDef {
        assert!(self.is_fn());
        let kind = self.0.kind();
        let _def = kind.fn_def().unwrap();
        (_def.0, _def.1.clone())
    }

    pub fn drop_instance(&self) -> Instance {
        Instance::resolve_drop_in_place(self.0)
    }

    pub fn function_instance(&self) -> Instance {
        let (def, args) = self.fn_def();
        Instance::resolve(def, &args).expect("Fail to instanlized function")
    }

    pub fn name(&self) -> NString {
        match self.0.kind().rigid().unwrap() {
            RigidTy::Bool => "bool".into(),
            RigidTy::Char => "char".into(),
            RigidTy::Int(i) => format!("{i:?}").to_lowercase().into(),
            RigidTy::Uint(i) => format!("{i:?}").to_lowercase().into(),
            RigidTy::Adt(def, _) => def.trimmed_name().into(),
            RigidTy::Array(ty, ..) => format!("Array({:?})", Type(*ty).name()).into(),
            RigidTy::Slice(ty) => format!("Slice({:?})", Type(*ty).name()).into(),
            RigidTy::RawPtr(ty, ..) => format!("Ptr({:?})", Type(*ty).name()).into(),
            RigidTy::Ref(_, ty, _) => format!("Ref({:?})", Type(*ty).name()).into(),
            RigidTy::Never => "never".into(),
            RigidTy::Tuple(f) => {
                if f.is_empty() {
                    "unit".into()
                } else {
                    let mut name = NString::from("_tuple");
                    for ty in f {
                        name += format!("_{:?}", Type(*ty).name());
                    }
                    name
                }
            }
            _ => todo!(),
        }
    }

    pub fn size(&self) -> usize {
        if self.is_unit() {
            return size_of::<()>();
        }

        if self.is_bool() {
            return size_of::<bool>();
        }

        if self.is_integer() {
            return match self.0.kind().rigid().unwrap() {
                RigidTy::Int(IntTy::I8) | RigidTy::Uint(UintTy::U8) => size_of::<u8>(),
                RigidTy::Int(IntTy::I16) | RigidTy::Uint(UintTy::U16) => size_of::<u16>(),
                RigidTy::Int(IntTy::I32) | RigidTy::Uint(UintTy::U32) => size_of::<u32>(),
                RigidTy::Int(IntTy::I64) | RigidTy::Uint(UintTy::U64) => size_of::<u64>(),
                RigidTy::Int(IntTy::I128) | RigidTy::Uint(UintTy::U128) => size_of::<u128>(),
                RigidTy::Int(IntTy::Isize) | RigidTy::Uint(UintTy::Usize) => size_of::<usize>(),
                _ => panic!("Impossible"),
            };
        }

        if self.is_array() {
            let len = self.array_len().unwrap() as usize;
            let elem_size = self.elem_type().size();
            return elem_size * len;
        }

        if self.is_primitive_ptr() {
            return size_of::<usize>();
        }

        // Do not support repr(align(N))
        if self.is_struct() || self.is_tuple() {
            let n = self.fields();
            let mut size = 0;
            for i in 0..n {
                let ty = self.field_type(i);
                let _align = ty.align();
                size = (size / _align + if size % _align != 0 { 1 } else { 0 }) * _align;
                size += ty.size();
            }
            let align = self.align();
            return (size / align + if size % align != 0 { 1 } else { 0 }) * align;
        }

        if self.is_enum() {
            // Discriminant size
            let discr_size = size_of::<isize>();
            let mut data_size = 1;
            for variant in self.enum_def().1 {
                let variant_size = variant.1[0].1.size();
                data_size = std::cmp::max(data_size, variant_size);
            }
            let size = discr_size + data_size;
            let align = self.align();
            return (size / align + if size % align != 0 { 1 } else { 0 }) * align;
        }

        todo!("{self:?}")
    }

    pub fn align(&self) -> usize {
        if self.is_unit() {
            return align_of::<()>();
        }

        if self.is_bool() {
            return align_of::<bool>();
        }

        if self.is_integer() {
            return match self.0.kind().rigid().unwrap() {
                RigidTy::Int(IntTy::I8) | RigidTy::Uint(UintTy::U8) => align_of::<u8>(),
                RigidTy::Int(IntTy::I16) | RigidTy::Uint(UintTy::U16) => align_of::<u16>(),
                RigidTy::Int(IntTy::I32) | RigidTy::Uint(UintTy::U32) => align_of::<u32>(),
                RigidTy::Int(IntTy::I64) | RigidTy::Uint(UintTy::U64) => align_of::<u64>(),
                RigidTy::Int(IntTy::I128) | RigidTy::Uint(UintTy::U128) => align_of::<u128>(),
                RigidTy::Int(IntTy::Isize) | RigidTy::Uint(UintTy::Usize) => align_of::<usize>(),
                _ => panic!("Impossible"),
            };
        }

        if self.is_array() {
            return self.elem_type().align();
        }

        if self.is_primitive_ptr() {
            return align_of::<usize>();
        }

        // Do not support repr(align(N))
        if self.is_struct() || self.is_tuple() {
            return (0..self.fields()).into_iter().fold(
                1,
                |acc, i|
                std::cmp::max(acc, self.field_type(i).align())
            );
        }

        if self.is_enum() {
            // Discriminant align
            let mut align = size_of::<isize>();
            for variant in self.enum_def().1 {
                align = std::cmp::max(align, variant.1[0].1.align());
            }
            return align;
        }

        todo!("{self:?}")
    }

    /// Reindex struct/tuple fields by eliminating prefix zero-sized type.
    pub fn fix_index_field(&self, i: &mut usize) {
        if self.is_array() || self.is_slice() { return; }
        let prefix_types = if self.is_struct() {
            self.struct_def().1.iter().map(|(_, ty)| *ty).collect::<Vec<_>>()
        } else {
            self.tuple_def()
        };
        *i -= prefix_types.iter().enumerate()
            .filter(|(j, _)| j < i)
            .fold(
                0,
                |acc, (j, ty)|
                acc + if ty.is_zero_sized_type() { 1 } else { 0 }
            );
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Ty> for Type {
    fn from(value: Ty) -> Self {
        assert!(matches!(value.kind(), TyKind::RigidTy(_)));
        Type(value)
    }
}

impl From<&Ty> for Type {
    fn from(value: &Ty) -> Self {
        Type::from(*value)
    }
}
