use std::{fmt::Debug, hash::Hash};

use stable_mir::mir::Local;

use super::nstring::NString;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ident {
    /// Global variable, `name`.
    Global(NString),
    /// Stack variable, `(function, frame_id, local)`.
    Stack(NString, usize, Local),
    /// Heap variable, `heap_object_<i>`.
    Heap(NString),
}

impl Ident {
    pub fn to_nstring(&self) -> NString {
        match self {
            Ident::Global(n) | Ident::Heap(n) => *n,
            Ident::Stack(func, frame, local) => format!("{func:?}_{frame}::{local}").into(),
        }
    }

    pub fn function(&self) -> NString {
        match self {
            Ident::Stack(func, ..) => *func,
            _ => panic!("Not stack symbol"),
        }
    }

    pub fn frame_id(&self) -> usize {
        match self {
            Ident::Stack(_, frame, ..) => *frame,
            _ => panic!("Not stack symbol"),
        }
    }

    pub fn local(&self) -> Local {
        match self {
            Ident::Stack(.., local) => *local,
            _ => panic!("Not stack symbol"),
        }
    }
}

impl Debug for Ident {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.to_nstring())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Level0,
    Level1,
    Level2,
}

/// Symbol are used for variables, objects, and so on.
///
/// `ident`: The original name of variable and heap objects.
/// where the identifiers of stack variables are in the form
/// of `FuncId::local` and the identifiers of heap variables
/// are in the form of `heap_object_{n}` such that n is the
/// unique id.
///
/// `l1_num`: Every time we encounter a `StorageLive`, we create a
///         fresh l1 symbol.
///
/// `l2_num`: Used for constructing verification condition(later used)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Symbol {
    ident: Ident,
    l1_num: usize,
    l2_num: usize,
    level: Level,
}

impl Symbol {
    pub fn new(ident: Ident, l1_num: usize, l2_num: usize, level: Level) -> Self {
        if level == Level::Level0 {
            assert!(l1_num == 0 && l2_num == 0);
        }
        if level == Level::Level1 {
            assert!(l1_num != 0 && l2_num == 0);
        }
        if level == Level::Level2 {
            assert!(l1_num != 0 && l2_num != 0);
        }
        Symbol { ident, l1_num, l2_num, level }
    }

    pub fn ident(&self) -> Ident {
        self.ident
    }

    pub fn function(&self) -> NString {
        assert!(self.is_stack_symbol());
        self.ident.function()
    }

    pub fn frame_id(&self) -> usize {
        assert!(self.is_stack_symbol());
        self.ident.frame_id()
    }

    pub fn local(&self) -> Local {
        assert!(self.is_stack_symbol());
        self.ident.local()
    }

    pub fn is_global_symbol(&self) -> bool {
        matches!(self.ident, Ident::Global(_))
    }

    pub fn is_stack_symbol(&self) -> bool {
        matches!(self.ident, Ident::Stack(..))
    }

    pub fn is_heap_symbol(&self) -> bool {
        matches!(self.ident, Ident::Heap(_))
    }

    pub fn is_level0(&self) -> bool {
        self.level == Level::Level0
    }

    pub fn is_level1(&self) -> bool {
        self.level == Level::Level1
    }

    pub fn is_level2(&self) -> bool {
        self.level == Level::Level2
    }

    pub fn l1_num(&self) -> usize {
        self.l1_num
    }

    pub fn l1_name(&self) -> NString {
        format!("{:?}::{}", self.ident, self.l1_num).into()
    }

    pub fn l2_name(&self) -> NString {
        format!("{:?}::{}", self.l1_name(), self.l2_num).into()
    }

    pub fn name(&self) -> NString {
        if self.is_level0() {
            self.ident().to_nstring()
        } else if self.is_level1() {
            self.l1_name()
        } else {
            self.l2_name()
        }
    }
}

impl Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.name()))
    }
}

impl From<Ident> for Symbol {
    fn from(value: Ident) -> Self {
        Symbol::new(value, 0, 0, Level::Level0)
    }
}

impl Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ident.hash(state);
        self.l1_num.hash(state);
        self.l2_num.hash(state);
        self.level.hash(state);
    }
}
