
pub(crate) enum Config {
  Z3(z3::Context),
}

impl Config {
  pub fn new(name: String) -> Self {
    assert!(name == "z3");

    Config::Z3(z3::Context::new(&z3::Config::new()))
  }

  pub fn to_z3_ctx(&self) -> &z3::Context {
    match self {
      Config::Z3(ctx) => ctx,
    }
  }
}