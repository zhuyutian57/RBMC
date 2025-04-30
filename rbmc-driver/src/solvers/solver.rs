use std::cell::RefCell;
use std::rc::Rc;

use crate::expr::expr::Expr;

use super::context::SolverCtx;
use super::smt::smt_conv::*;
use super::z3::z3_conv::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PResult {
    PSat,
    PUnknow,
    PUnsat,
}

pub struct Solver<'ctx> {
    smt_solver: Box<dyn SmtSolver<'ctx> + 'ctx>,
}

impl<'ctx> Solver<'ctx> {
    pub fn new(solver_ctx: &'ctx SolverCtx) -> Self {
        let mut smt_solver = match solver_ctx {
            SolverCtx::Z3(ctx) => Box::new(Z3Conv::new(ctx)),
        };
        smt_solver.init();
        Solver { smt_solver }
    }

    pub fn check(&self) -> PResult {
        self.smt_solver.check()
    }

    pub fn push(&self) {
        self.smt_solver.push();
    }

    pub fn pop(&self) {
        self.smt_solver.pop();
    }

    pub fn reset(&mut self) {
        self.smt_solver.reset();
    }

    pub fn eval_bool(&self, expr: Expr) -> bool {
        assert!(expr.ty().is_bool());
        self.smt_solver.eval_bool(expr)
    }

    pub fn show_model(&self) {
        println!("Model:");
        self.smt_solver.show_model();
    }

    pub fn assert_assign(&mut self, lhs: Expr, rhs: Expr) {
        self.smt_solver.assert_assign(lhs, rhs);
    }

    pub fn assert_expr(&mut self, expr: Expr) {
        self.smt_solver.assert_expr(expr);
    }
}
