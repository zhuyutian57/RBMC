
use std::{fmt::Debug, hash::Hash};

use super::nstring::NString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
  level0,
  level1,
  level2,
}

/// Symbol are used for variables, objects, and so on.
/// 
/// identifier: The original name of variable and heap objects.
///             Usually, it is constructed by function name,
///             frame number and local number. 
/// 
/// l1_num: Every time we encounter a `StorageLive`, we create a
///         fresh l1 symbol.
/// 
/// l2_num: Used for constructing verification condition(later used)
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Symbol {
  identifier: NString,
  l1_num: usize,
  l2_num: usize,
  level: Level,
}

impl Symbol {
  pub fn new(
    identifier: NString,
    l1_num: usize,
    l2_num: usize,
    level: Level
  ) -> Self {
    Symbol { identifier, l1_num, l2_num, level }
  }

  pub fn identifier(&self) -> NString { self.identifier.clone() }

  pub fn is_level0(&self) -> bool { self.level == Level::level0 }
  pub fn is_level1(&self) -> bool { self.level == Level::level1 }
  pub fn is_level2(&self) -> bool { self.level == Level::level2 }

  pub fn l1_name(&self) -> NString {
    self.identifier + "::" + self.l1_num.to_string()
  }

  pub fn l2_name(&self) -> NString {
    self.l1_name() + "::" + self.l2_num.to_string()
  }

  pub fn name(&self) -> NString {
    if self.is_level0() { self.identifier() }
    else if self.is_level1() { self.l1_name() }
    else { self.l2_name() }
  }
}

impl Debug for Symbol {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("{:?}", self.name()))
  }
}

impl Hash for Symbol {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.identifier.hash(state);
    if self.level == Level::level1 {
      self.l1_num.hash(state);
    }
    if self.level == Level::level2 {
      self.l1_num.hash(state);
      self.l2_num.hash(state);
    }
  }
}