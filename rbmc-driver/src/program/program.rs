use std::collections::HashMap;
use std::io::*;

use num_bigint::BigInt;
use num_bigint::Sign;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::mono::StaticDef;
use stable_mir::mir::TerminatorKind;
use stable_mir::target::*;
use stable_mir::ty::FnDef;
use stable_mir::*;

use super::function::*;
use crate::config::cli::ProgramInfo;
use crate::expr::ty::Type;
use crate::symbol::nstring::NString;

pub struct Program {
    name: NString,
    static_variables: Vec<StaticDef>,
    /// The number of functions in current crate
    func_count: usize,
    functions: Vec<Function>,
    function_map: HashMap<NString, FunctionIdx>,
}

impl Program {
    pub fn new(_crate: Crate, entry_function: NString) -> Self {
        let mut functions = Vec::new();
        _crate.fn_defs().iter().for_each(
            |def| functions.push(Function::from(def))
        );
        let mut idx = HashMap::new();
        functions.iter().enumerate().for_each(
            |(i, function)|
            { idx.insert(function.name(), i); }
        );
        assert!(idx.contains_key(&entry_function));
        let mut program = Program {
            name: _crate.name.clone().into(),
            static_variables: _crate.statics(),
            func_count: functions.len(),
            functions: functions,
            function_map: idx
        };
        program.init();
        program
    }

    fn init(&mut self) {
        // Cache all reachable funtions
        let mut i = 0;
        while i < self.functions.len() {
            let function = &self.functions[i];
            let locals = function.locals();
            let mut new_functions = Vec::new();
            for bb in function.body().blocks.iter() {
                let instance = match &bb.terminator.kind {
                    TerminatorKind::Drop { place, .. } => {
                        let ty = place.ty(locals).unwrap();
                        Some(Instance::resolve_drop_in_place(ty))
                    },
                    TerminatorKind::Call { func, .. } => {
                        let ty = func.ty(locals).unwrap();
                        let k = ty.kind();
                        let (def, args) = k.fn_def().unwrap();
                        let instance = Instance::resolve(def, args).expect("Not compile?");
                        if instance.has_body() {
                            Some(instance)
                        } else {
                            None
                        }
                    },
                    _ => None, // Do nothing
                };
                if let Some(inst) = instance {
                    if Type::from(inst.ty()).is_builtin_function() { continue; }
                    let name = NString::from(inst.trimmed_name());
                    if self.function_map.contains_key(&name) { continue; }
                    let idx = self.functions.len() + new_functions.len();
                    self.function_map.insert(name, idx);
                    new_functions.push(Function::from(&inst));
                }
            }
            new_functions.into_iter().for_each(|function| self.functions.push(function));
            i += 1;
        }
    }

    pub fn static_variables(&self) -> &Vec<StaticDef> {
        &self.static_variables
    }

    pub fn contains_function(&self, name: NString) -> bool {
        self.function_map.contains_key(&name)
    }

    pub fn function_id(&self, name: NString) -> FunctionIdx {
        assert!(self.contains_function(name));
        *self.function_map.get(&name).unwrap()
    }


    pub fn function(&self, i : FunctionIdx) -> &Function {
        assert!(i < self.functions.len());
        &self.functions[i]
    }

    pub fn size(&self) -> usize {
        self.functions.len()
    }

    pub fn show(&self, info: ProgramInfo) {
        let target = MachineInfo::target();
        println!(
            "Crate:{:?}, Endian:{}, MachineSize:{}\n",
            self.name,
            match target.endian {
                Endian::Little => "Little",
                _ => "Big",
            },
            target.pointer_width.bytes()
        );
        let n = if info == ProgramInfo::Local {
            self.func_count
        } else {
            self.functions.len()
        };
        for i in 0..n {
            let function = &self.functions[i];
            println!("--->>> Function: {:?}", function.name());
            function.show();
            println!("<<<--- End: {:?}\n", function.name());
        }
    }
}

pub(crate) fn read_target_integer(bytes: &[u8]) -> BigInt {
    match MachineInfo::target().endian {
        Endian::Big => BigInt::from_signed_bytes_be(bytes),
        Endian::Little => BigInt::from_signed_bytes_le(bytes),
    }
}

pub fn bigint_to_u64(bigint: &BigInt) -> u64 {
    if bigint == &BigInt::ZERO {
        return 0;
    }
    let (sign, digits) = bigint.to_u64_digits();
    assert!(sign == Sign::NoSign || sign == Sign::Plus);
    assert!(digits.len() == 1);
    digits[0]
}

pub fn bigint_to_usize(bigint: &BigInt) -> usize {
    bigint_to_u64(bigint) as usize
}
