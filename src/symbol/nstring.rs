
use std::alloc::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, AddAssign};

/// Used to manage String. Reduce allocation for String
struct StringManager {
  strings: Vec<String>,
  map: HashMap<String, usize>,
}

impl StringManager {
  fn new() -> Self {
    let strings =
      vec![
        "".to_string(),
        "alloc".to_string(),
        "INVALID-OBJECT".to_string(),
      ];
    let mut map = HashMap::new();
    map.insert("".to_string(), 0);
    map.insert("alloc".to_string(), 1);
    map.insert("INVALID-OBJECT".to_string(), 2);
    StringManager { strings, map }
  }

  fn get_string(&self, i: usize) -> &String {
    assert!(i < self.strings.len());
    &self.strings[i]
  }

  fn get_id(&mut self, s: &String) -> usize {
    if !self.map.contains_key(s) {
      self.strings.push(s.clone());
      self.map.insert(s.clone(), self.strings.len() - 1); 
    }
    *self.map.get(s).expect("Do not exists")
  }
}

/// The global manager for String.
static mut STRING_M : *mut StringManager = std::ptr::null_mut();

fn string_m() -> &'static mut StringManager {
  unsafe {
    if STRING_M.is_null() {
      STRING_M = alloc(Layout::new::<StringManager>()) as *mut StringManager;
      std::ptr::write(STRING_M, StringManager::new());
    }
    
    &mut *STRING_M
  }
}

/// A wrapper for String
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NString(usize);

impl NString {
  pub const EMPTY: NString = NString(0);
  pub const ALLOC_SYM: NString = NString(1);
  pub const INVALID_OBJECT: NString = NString(2);

  pub fn is_empty(&self) -> bool {
    *self == NString::EMPTY
  }

  pub fn contains(&self, str: NString) -> bool {
    let string = string_m().get_string(self.0);
    let sub_str = string_m().get_string(str.0);
    string.contains(sub_str)
  }

  pub fn find(&self, s: NString) -> Option<usize> {
    self.to_string().find(s.as_str())
  }

  pub fn sub_str(&self, l: usize, r: usize) -> NString {
    let str = self.as_str();
    assert!(0 <= l && l < r && r <= str.len());
    NString::from(&str[l..r])
  }
  
  /// During the symex, we do not delete any NString.
  /// Thus, returning static str is safe.
  pub fn as_str(&self) -> &'static str {
    string_m().get_string(self.0).as_str()
  }
}

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

impl AddAssign for NString {
  fn add_assign(&mut self, rhs: Self) {
    self.0 =
      NString::from(
        *self + string_m().get_string(rhs.0).as_str()
      ).0
  }
}

impl AddAssign<String> for NString {
  fn add_assign(&mut self, rhs: String) {
    self.0 = (*self + rhs).0
  }
}

impl AddAssign<&str> for NString {
  fn add_assign(&mut self, rhs: &str) {
    self.0 = (*self + rhs).0
  }
}


impl Debug for NString {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", string_m().get_string(self.0))
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