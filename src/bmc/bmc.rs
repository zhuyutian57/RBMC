
use std::cell::RefCell;

use crate::config::cli::SmtStrategy;
use crate::expr::expr::ExprBuilder;
use crate::solvers::solver::*;
use crate::symex::symex::*;
use crate::vc::slicer::Slicer;
use crate::vc::vc::*;
use crate::Config;

pub struct Bmc<'cfg> {
  config: &'cfg Config,
  symex: Symex<'cfg>,
  vc_system: VCSysPtr,
  runtime_solver: Solver<'cfg>,
}

impl<'cfg> Bmc<'cfg> {
  pub fn new(config: &'cfg Config) -> Self {
    let vc_system =
      VCSysPtr::new(RefCell::new(VCSystem::default()));
    let symex = Symex::new(config, vc_system.clone());
    let runtime_solver =
      Solver::new(&config.solver_config);
    Bmc { config, symex, vc_system, runtime_solver }
  }

  pub fn do_bmc(&mut self) {
    if self.config.cli.show_program {
      self.config.program.show();
    }

    self.symex.run();

    self.check_properties();
  }

  fn check_properties(&mut self) {
    let res =
      match self.config.cli.smt_strategy {
        SmtStrategy::Forward => self.check_forward(),
        SmtStrategy::Once => self.check_once(),
      };
    println!(
      "Verification result: {}.",
      match res {
        PResult::PSat => "fail",
        PResult::PUnknow => "unknown",
        PResult::PUnsat => "success",
      }
    );
  }

  fn check_forward(&mut self) -> PResult {
    let mut slicer = Slicer::default();
    let size = self.vc_system.borrow().num_asserts();
    for i in 0..size {
      print!("Verifying condition {i} ");

      self.vc_system.borrow_mut().set_nth_assertion(i);

      if !self.config.cli.no_slice {
        slicer.slice_nth(self.vc_system.clone(), i);
      }

      if self.config.cli.show_vcc {
        self.vc_system.borrow().show_vcc();
      }
      
      self.runtime_solver.reset();
      self.generate_smt_formula();
      
      let res = self.smt_result();
      println!("Result: {res:?}\n");
      if res != PResult::PUnsat { return res; }
    }
    PResult::PUnsat
  }

  fn check_once(&mut self) -> PResult {
    println!("Verifying condition:");
    if !self.config.cli.no_slice {
      let mut slicer = Slicer::default();
      slicer.slice_whole(self.vc_system.clone());
    }

    if self.config.cli.show_vcc {
      self.vc_system.borrow().show_vcc();
    }

    self.runtime_solver.reset();
    self.generate_smt_formula();

    let res = self.smt_result();
    println!("Result: {res:?}\n");
    res
  }

  fn smt_result(&mut self) -> PResult {
    let res = self.runtime_solver.check();
    if res == PResult::PSat && self.config.cli.show_smt_model {
      self.runtime_solver.show_model();
    }
    res
  }

  fn generate_smt_formula(&mut self) {
    let ctx = self.config.expr_ctx.clone();

    let mut assumetion = ctx._true();
    let mut assertions = Vec::new();
    
    for vc in self.vc_system.borrow().iter() {
      if vc.is_sliced { continue; }
      match &vc.kind {
        VcKind::Assign(lhs, rhs) => {
          self.runtime_solver.assert_assign(lhs.clone(), rhs.clone());
        },
        VcKind::Assert(_, c) => {
          assertions.push(ctx.implies(assumetion.clone(), c.clone()));
        },
        VcKind::Assume(c) => {
          assumetion = ctx.and(assumetion, c.clone());
          assumetion.simplify();
        },
      }
    }

    self.runtime_solver.assert_expr(
      if assertions.is_empty() { ctx._false() }
      else {
        let mut assertion =
          assertions.into_iter().fold(
            ctx._false(),
            |acc, b| ctx.or(acc, b)
          );
        assertion.simplify();
        assertion
      }
    );
  }
}