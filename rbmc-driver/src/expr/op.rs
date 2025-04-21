use std::fmt::Debug;
use std::fmt::Error;

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
        }
    }
}

impl From<mir::BinOp> for BinOp {
    fn from(value: mir::BinOp) -> Self {
        match value {
            mir::BinOp::Add | mir::BinOp::AddUnchecked => Ok(BinOp::Add),
            mir::BinOp::Sub | mir::BinOp::SubUnchecked => Ok(BinOp::Sub),
            mir::BinOp::Mul | mir::BinOp::MulUnchecked => Ok(BinOp::Mul),
            mir::BinOp::Div => Ok(BinOp::Div),
            mir::BinOp::Eq => Ok(BinOp::Eq),
            mir::BinOp::Ne => Ok(BinOp::Ne),
            mir::BinOp::Le => Ok(BinOp::Le),
            mir::BinOp::Lt => Ok(BinOp::Lt),
            mir::BinOp::Ge => Ok(BinOp::Ge),
            mir::BinOp::Gt => Ok(BinOp::Gt),
            mir::BinOp::BitAnd => Ok(BinOp::And),
            mir::BinOp::BitOr => Ok(BinOp::Or),
            _ => Err(Error),
        }
        .expect("Do not support")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnOp {
    Not,
    Neg,
    Meta,
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
