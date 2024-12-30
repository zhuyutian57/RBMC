
use std::{cell::LazyCell, collections::HashMap, fmt::Debug, ops::Add};

/// Used to manage String. Reduce allocation for String
#[derive(Default)]
struct StringManager {
  strings: Vec<String>,
  string_map: HashMap<String, usize>,
}

impl StringManager {
  fn get_string(&self, i: usize) -> &String {
    assert!(i < self.strings.len());
    &self.strings[i]
  }

  fn get_id(&mut self, s: &String) -> usize {
    if !self.string_map.contains_key(s) {
      self.strings.push(s.clone());
      self.string_map.insert(s.clone(), self.strings.len() - 1); 
    }
    *self.string_map.get(s).expect("Do not exists")
  }
}

/// The global manager for String.
static mut STRING_M : LazyCell<StringManager>
  = LazyCell::new(|| StringManager::default());

fn string_m() -> &'static mut StringManager {
  unsafe { LazyCell::force_mut(&mut STRING_M) }
}

/// A wrapper for String
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NString(usize);

impl PartialEq<&str> for NString {
  fn eq(&self, other: &&str) -> bool {
    *self == NString::from(*other)
  }
}

impl Add for NString {
  type Output = Self;
  fn add(self, rhs: Self) -> Self {
    NString::from(
      self + string_m().get_string(rhs.0).as_str()
    )
  }
}

impl Add<String> for NString {
  type Output = Self;
  fn add(self, rhs: String) -> Self::Output {
    self + rhs.as_str()
  }
}

impl Add<&str> for NString {
  type Output = Self;
  fn add(self, rhs: &str) -> Self::Output {
    let new_string =
      string_m().get_string(self.0).clone() + rhs;
    NString(string_m().get_id(&new_string))
  }
}

impl Debug for NString {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", unsafe {
      STRING_M.get_string(self.0)
    })
  }
}

impl From<String> for NString {
  fn from(value: String) -> Self {
    NString(string_m().get_id(&value))
  }
}

impl From<&str> for NString {
  fn from(value: &str) -> Self {
    NString::from(value.to_string())
  }
}

impl ToString for NString {
  fn to_string(&self) -> String {
    string_m().get_string(self.0).clone()
  }
}