
use stable_mir::mir::Place;

use crate::expr::expr::*;
use super::state::*;

/// Dereferencing a place
pub(super) struct Projector<'sym, 'frame> {
  frame: &'sym mut Frame<'frame>,
}

impl<'sym, 'frame> Projector<'sym, 'frame> {
  pub fn new(frame: &'sym mut Frame<'frame>) -> Self {
    Projector { frame }
  }

  pub fn projecting(&mut self, place: &Place) -> Expr {
    todo!()
  }
}