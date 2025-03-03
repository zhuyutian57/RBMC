
/// The predicate for Ownership
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ownership {
  /// `Own(x)` means that some variables owns `x`.
  Own,
  /// `Not` owned by any variable.
  Not,
  /// `Unknown`
  Unknown,
}

impl Ownership {
  pub fn is_own(&self) -> bool { matches!(self, Ownership::Own) }
  
  pub fn is_not(&self) -> bool { matches!(self, Ownership::Not) }

  pub fn is_unknown(&self) -> bool { matches!(self, Ownership::Unknown) }
}