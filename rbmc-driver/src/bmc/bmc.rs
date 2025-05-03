use std::cell::RefCell;

use crate::config::cli::SmtStrategy;
use crate::config::config::Config;
use crate::expr::expr::ExprBuilder;
use crate::solvers::solver::*;
use crate::symex::symex::*;
use crate::vc::slicer::Slicer;
use crate::vc::vc::*;

pub struct Bmc<'cfg> {
    config: &'cfg Config,
    symex: Symex<'cfg>,
    vc_system: VCSysPtr,
    runtime_solver: Solver<'cfg>,
}

impl<'cfg> Bmc<'cfg> {
    pub fn new(config: &'cfg Config) -> Self {
        let vc_system = VCSysPtr::new(RefCell::new(VCSystem::new(config.expr_ctx.clone())));
        let symex = Symex::new(config, vc_system.clone());
        let runtime_solver = Solver::new(&config.solver_config);
        Bmc { config, symex, vc_system, runtime_solver }
    }

    pub fn do_bmc(&mut self) {
        if self.config.cli.show_program || self.config.cli.program_only {
            self.config.program.show(!self.config.cli.show_std_function);
            if self.config.cli.program_only {
                return;
            }
        }

        let verify_time = std::time::Instant::now();
        println!("Start Symex ...");

        self.symex.run();
        println!("Runtime Symex: {}s", verify_time.elapsed().as_secs_f32());

        self.vc_system.borrow().show_info();

        let res = if self.vc_system.borrow().num_asserts() == 0 {
            println!("No assertions should be checked");
            PResult::PUnsat
        } else {
            self.check_properties()
        };

        println!("\nVerification time: {}s", verify_time.elapsed().as_secs_f32());
        println!(
            "Verification result: {}.",
            match res {
                PResult::PSat => "fail",
                PResult::PUnknow => "unknown",
                PResult::PUnsat => "success",
            }
        );
    }

    fn check_properties(&mut self) -> PResult {
        println!("Verifying with SMT strategy: {:?}", self.config.cli.smt_strategy);
        let (res, bug) = match self.config.cli.smt_strategy {
            SmtStrategy::Forward => self.check_forward(),
            SmtStrategy::Once => (self.check_once(), None),
        };
        if res == PResult::PSat {
            self.bug_report(bug);
        }
        res
    }

    fn check_forward(&mut self) -> (PResult, Option<usize>) {
        let mut slicer = Slicer::default();
        let size = self.vc_system.borrow().num_asserts();
        for i in 0..size {
            println!("Begin checking assertion {i}");
            if self.config.cli.show_vcc {
                print!("Verifying condition {i} ");
            }

            self.vc_system.borrow_mut().set_nth_assertion(i);

            if !self.config.cli.no_slice {
                let slice_time = std::time::Instant::now();
                slicer.slice_nth(self.vc_system.clone(), i);
                println!("Runtime slicing asssertion {i}: {}s", slice_time.elapsed().as_secs_f32());
                println!("After slicing: {} VC(s)", self.vc_system.borrow().num_valid_vc());
            }

            if self.config.cli.show_vcc {
                self.vc_system.borrow().show_vcc();
            }

            self.runtime_solver.reset();
            let convert_time = std::time::Instant::now();
            self.generate_smt_formula();
            println!("Runtime Convert SSA: {}s", convert_time.elapsed().as_secs_f32());

            let solver_time = std::time::Instant::now();
            let res = self.smt_result();
            println!("Runtime SMT check: {}s", solver_time.elapsed().as_secs_f32());
            if self.config.cli.show_vcc {
                println!("Result: {res:?} ");
            }
            match res {
                PResult::PSat => return (res, Some(i)),
                PResult::PUnknow => return (res, None),
                _ => {}
            }
        }
        (PResult::PUnsat, None)
    }

    fn check_once(&mut self) -> PResult {
        println!("Begin checking all assertions at once");
        if self.config.cli.show_vcc {
            println!("Verifying condition:");
        }
        if !self.config.cli.no_slice {
            let mut slicer = Slicer::default();
            let slice_time = std::time::Instant::now();
            slicer.slice_whole(self.vc_system.clone());
            println!("Runtime slicing asssertion: {}s", slice_time.elapsed().as_secs_f32());
            println!("After slicing: {} VC(s)", self.vc_system.borrow().num_valid_vc());
        }

        if self.config.cli.show_vcc {
            self.vc_system.borrow().show_vcc();
        }

        self.runtime_solver.reset();
        let convert_time = std::time::Instant::now();
        self.generate_smt_formula();
        println!("Runtime Convert SSA: {}s", convert_time.elapsed().as_secs_f32());

        let solver_time = std::time::Instant::now();
        let res = self.smt_result();
        println!("Runtime SMT check: {}s", solver_time.elapsed().as_secs_f32());
        if self.config.cli.show_vcc {
            print!("Result: {res:?} ");
        }
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
        println!("Converting SSA");
        let ctx = self.config.expr_ctx.clone();

        // let mut assume = ctx._true();
        let mut assertions = Vec::new();

        for vc in self.vc_system.borrow().iter() {
            if vc.is_sliced {
                continue;
            }
            match &vc.kind {
                VcKind::Assign(lhs, rhs) => {
                    self.runtime_solver.assert_assign(lhs.clone(), rhs.clone());
                }
                VcKind::Assert(_, c) => {
                    assertions.push(c.clone());
                    // assertions.push(ctx.implies(assume.clone(), c.clone()));
                }
                VcKind::Assume(c) => {
                    self.runtime_solver.assert_expr(c.clone());
                    // assume = ctx.and(assume, c.clone());
                }
            }
        }

        let assert = assertions.into_iter().fold(ctx._false(), |acc, b| ctx.or(acc, b));

        self.runtime_solver.assert_expr(assert);
    }

    fn bug_report(&self, bug: Option<usize>) {
        println!("\nBug Report:");
        if self.config.cli.smt_strategy == SmtStrategy::Forward {
            let assertion = self.vc_system.borrow().nth_assertion(bug.unwrap());
            Bmc::bug_info(&assertion);
        } else {
            for n in 0..self.vc_system.borrow().num_asserts() {
                let assertion = self.vc_system.borrow().nth_assertion(n);
                if self.runtime_solver.eval_bool(assertion.cond()) {
                    Bmc::bug_info(&assertion);
                    // Only show the first violated property
                    break;
                }
            }
        }
        println!("");
    }

    #[inline]
    fn bug_info(assertion: &Vc) {
        let span = assertion.span.expect("Span must exist");
        println!(
            "-> {}:{}:{}: {:?}",
            span.get_filename(),
            span.get_lines().start_line,
            span.get_lines().start_col,
            assertion.msg()
        );
    }
}
