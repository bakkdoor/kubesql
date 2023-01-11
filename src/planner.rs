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

#[derive(Debug)]
pub enum Value {
    Strings(Vec<String>),
    String(String),
    Query(Query),
    Queries(Vec<Query>),
}

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum PlanError {
    #[allow(dead_code)]
    #[error("Unknown PlanError: {0}")]
    Unknown(String),
}

type PlanResult = Result<Value, PlanError>;

pub(crate) fn plan_expr(expr: ast::Expr) -> PlanResult {
    match expr {
        ast::Expr::CompoundIdentifier(i) => plan_expr_compound_ident(&i),
        ast::Expr::BinaryOp { left, op, right } => plan_expr_binary_op(*left, op, *right),
        ast::Expr::Value(v) => plan_expr_value(v),
        _ => {
            panic!("plan_expr::unsupported: {:?}", expr);
        }
    }
}

fn plan_expr_compound_ident(idents: &[ast::Ident]) -> PlanResult {
    Ok(Value::Strings(
        idents.iter().cloned().map(|e| e.value).collect(),
    ))
}

fn plan_expr_binary_op(left: ast::Expr, op: ast::BinaryOperator, right: ast::Expr) -> PlanResult {
    let l = plan_expr(left)?;
    let r = plan_expr(right)?;

    match (l, r) {
        (Value::Strings(a), Value::String(b)) => plan_expr_binary_op_query(a, b, op),
        (Value::Query(a), Value::Query(b)) => plan_expr_binary_op_query_vec(a, b, op),
        (Value::Queries(a), Value::Query(b)) => plan_expr_binary_op_query_vec_append(a, b, op),
        (x, y) => {
            panic!("Type mismatch L: {:?}, R: {:?}!", x, y)
        }
    }
}

fn plan_expr_value(value: ast::Value) -> PlanResult {
    match value {
        ast::Value::SingleQuotedString(s) | ast::Value::DoubleQuotedString(s) => {
            Ok(Value::String(s))
        }
        _ => {
            panic!("plan_expr_value::unsupported!")
        }
    }
}

fn plan_expr_binary_op_query(
    input: Vec<String>,
    eq: String,
    op: ast::BinaryOperator,
) -> PlanResult {
    if input.len() != 3 {
        panic!("WHERE statement does only support three length CompoundIdentifier: i.e. 'pod.status.phase'")
    }

    Ok(Value::Query(Query {
        key: None,
        kind: input.get(0).unwrap().to_string(),
        field1: input.get(1).unwrap().to_string(),
        field2: input.get(2).unwrap().to_string(),
        eq: eq.replace('_', "-"),
        op,
    }))
}

fn plan_expr_binary_op_query_vec(
    input: Query,
    mut eq: Query,
    op: ast::BinaryOperator,
) -> PlanResult {
    let mut v = vec![input];
    eq.key = Some(op);
    v.push(eq);

    Ok(Value::Queries(v))
}

fn plan_expr_binary_op_query_vec_append(
    input: Vec<Query>,
    mut eq: Query,
    op: ast::BinaryOperator,
) -> PlanResult {
    let mut v = input;
    eq.key = Some(op);
    v.push(eq);

    Ok(Value::Queries(v))
}
