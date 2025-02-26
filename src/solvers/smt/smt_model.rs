
use std::fmt::Debug;

pub enum SmtModel<'ctx> {
  Z3Model(z3::Model<'ctx>),
}

impl<'ctx> Debug for SmtModel<'ctx> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      SmtModel::Z3Model(m) => write!(f, "{m:?}"),
    }
  }
}
