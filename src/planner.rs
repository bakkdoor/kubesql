// Copyright (c) 2021 Dentrax
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

use sqlparser::ast;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Query {
    pub key: Option<ast::BinaryOperator>,
    pub kind: String,
    pub field1: String,
    pub field2: String,
    pub eq: String,
    pub op: ast::BinaryOperator,
}

#[derive(Debug, Clone)]
pub enum Value {
    Strings(Vec<String>),
    String(String),
    Query(Query),
    Queries(Vec<Query>),
}

#[derive(Error, Debug, Clone)]
pub enum PlanError {
    #[error("Unknown PlanError: {0}")]
    Unknown(String),

    #[error("PlanQuery for {0} unsupported: {1}")]
    Unsupported(String, String),

    #[error("Type mismatch L: {0:?}, R: {1:?}!")]
    TypeMismatch(Box<Value>, Box<Value>),
}

type PlanResult = Result<Value, PlanError>;

#[derive(Debug, Clone, Default)]
pub struct PlanContext {
    // todo
}

pub trait PlanQuery {
    fn plan(&self, context: &mut PlanContext) -> PlanResult;
}

impl PlanQuery for ast::Expr {
    fn plan(&self, context: &mut PlanContext) -> PlanResult {
        match self {
            ast::Expr::Value(v) => v.plan(context),
            ast::Expr::CompoundIdentifier(identifiers) => {
                CompoundIdentifier { identifiers }.plan(context)
            }
            ast::Expr::BinaryOp { left, op, right } => BinaryOp { left, op, right }.plan(context),
            _ => Err(PlanError::Unsupported("Expr".to_string(), self.to_string())),
        }
    }
}

impl PlanQuery for ast::Value {
    fn plan(&self, _context: &mut PlanContext) -> PlanResult {
        match self {
            ast::Value::SingleQuotedString(s) | ast::Value::DoubleQuotedString(s) => {
                Ok(Value::String(s.clone()))
            }
            _ => Err(PlanError::Unsupported(
                "Value".to_string(),
                self.to_string(),
            )),
        }
    }
}

struct CompoundIdentifier<'a> {
    identifiers: &'a [ast::Ident],
}

impl<'a> PlanQuery for CompoundIdentifier<'a> {
    fn plan(&self, _context: &mut PlanContext) -> PlanResult {
        Ok(Value::Strings(
            self.identifiers.iter().cloned().map(|e| e.value).collect(),
        ))
    }
}

struct BinaryOp<'a> {
    op: &'a ast::BinaryOperator,
    left: &'a ast::Expr,
    right: &'a ast::Expr,
}

impl<'a> PlanQuery for BinaryOp<'a> {
    fn plan(&self, context: &mut PlanContext) -> PlanResult {
        let l = self.left.plan(context)?;
        let r = self.right.plan(context)?;

        match (l, r) {
            (Value::Strings(a), Value::String(b)) => BinaryOpQuery {
                op: self.op,
                input: &a,
                eq: &b,
            }
            .plan(context),
            (Value::Query(input), Value::Query(mut eq)) => {
                let mut v = vec![input];
                eq.key = Some(self.op.clone());
                v.push(eq);

                Ok(Value::Queries(v))
            }
            (Value::Queries(input), Value::Query(mut eq)) => {
                let mut v = input;
                eq.key = Some(self.op.clone());
                v.push(eq);
                Ok(Value::Queries(v))
            }
            (x, y) => Err(PlanError::TypeMismatch(Box::new(x), Box::new(y))),
        }
    }
}

struct BinaryOpQuery<'a> {
    op: &'a ast::BinaryOperator,
    input: &'a [String],
    eq: &'a String,
}

impl<'a> PlanQuery for BinaryOpQuery<'a> {
    fn plan(&self, _context: &mut PlanContext) -> PlanResult {
        if self.input.len() != 3 {
            return Err(PlanError::Unknown("WHERE statement does only support three length CompoundIdentifier: i.e. 'pod.status.phase'".to_string()));
        }

        Ok(Value::Query(Query {
            key: None,
            kind: self.input.get(0).unwrap().to_string(),
            field1: self.input.get(1).unwrap().to_string(),
            field2: self.input.get(2).unwrap().to_string(),
            eq: self.eq.replace('_', "-"),
            op: self.op.clone(),
        }))
    }
}
