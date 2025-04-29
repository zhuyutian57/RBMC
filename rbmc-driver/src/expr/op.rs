use std::fmt::Debug;

use stable_mir::mir;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Ge,
    Gt,
    Le,
    Lt,
    And,
    Or,
    Implies,
    Offset,
}

impl Debug for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Eq => write!(f, "=="),
            BinOp::Ne => write!(f, "!="),
            BinOp::Ge => write!(f, ">="),
            BinOp::Gt => write!(f, ">"),
            BinOp::Le => write!(f, "<="),
            BinOp::Lt => write!(f, "<"),
            BinOp::And => write!(f, "&&"),
            BinOp::Or => write!(f, "||"),
            BinOp::Implies => write!(f, "=>"),
            BinOp::Offset => write!(f, "Offset"),
        }
    }
}

impl From<mir::BinOp> for BinOp {
    fn from(value: mir::BinOp) -> Self {
        match value {
            mir::BinOp::Add | mir::BinOp::AddUnchecked => BinOp::Add,
            mir::BinOp::Sub | mir::BinOp::SubUnchecked => BinOp::Sub,
            mir::BinOp::Mul | mir::BinOp::MulUnchecked => BinOp::Mul,
            mir::BinOp::Div => BinOp::Div,
            mir::BinOp::Eq => BinOp::Eq,
            mir::BinOp::Ne => BinOp::Ne,
            mir::BinOp::Le => BinOp::Le,
            mir::BinOp::Lt => BinOp::Lt,
            mir::BinOp::Ge => BinOp::Ge,
            mir::BinOp::Gt => BinOp::Gt,
            mir::BinOp::BitAnd => BinOp::And,
            mir::BinOp::BitOr => BinOp::Or,
            mir::BinOp::Offset => BinOp::Offset,
            _ => todo!("{value:?}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Not,
    Neg,
    Meta,
}

impl UnOp {
    pub fn is_not(&self) -> bool {
        matches!(self, UnOp::Not)
    }
    
    pub fn is_neg(&self) -> bool {
        matches!(self, UnOp::Neg)
    }
    
    pub fn is_meta(&self) -> bool {
        matches!(self, UnOp::Meta)
    }
}

impl Debug for UnOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnOp::Not => write!(f, "!"),
            UnOp::Neg => write!(f, "-"),
            UnOp::Meta => write!(f, "meta"),
        }
    }
}

impl From<mir::UnOp> for UnOp {
    fn from(value: mir::UnOp) -> Self {
        match value {
            mir::UnOp::Not => UnOp::Not,
            mir::UnOp::Neg => UnOp::Neg,
            mir::UnOp::PtrMetadata => UnOp::Meta,
        }
    }
}
