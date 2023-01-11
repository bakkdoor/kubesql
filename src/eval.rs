use crate::planner::Value;

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum EvalError {
    #[allow(dead_code)]
    #[error("Unknown EvalError: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct EvalContext {}

#[allow(dead_code)]
pub type EvalResult = Result<Value, EvalError>;

pub trait Evaluate {
    fn evaluate(&self, context: &mut EvalContext) -> EvalResult;
}
