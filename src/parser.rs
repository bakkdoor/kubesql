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

use crate::planner::{self, PlanQuery};
use crate::planner::{Query, Value};
use kube::config::{Kubeconfig, KubeconfigError};
use sqlparser::ast::{SelectItem, SetExpr, Statement, TableFactor};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("ParserError")]
pub enum ParserError {
    #[allow(dead_code)]
    #[error("Unknown: {0}")]
    Unknown(String),

    #[error("Unsupported: {0}")]
    Unsupported(String),

    #[error("KubeConfigError: {0:?}")]
    KubeConfigError(KubeconfigError),

    #[error("SELECT statement is required to call the given namespace(s)!")]
    SelectProjectionsRequired,

    #[error("FROM statement is required to call the given context(s)!")]
    SelectFromRequired,
}

#[derive(Debug)]
pub struct ApiQueries {
    pub namespaces: Vec<String>,
    pub contexts: Vec<String>,
    pub queries: Vec<Query>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum ResourceType {
    Deployment,
    Pod,
    Service,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Deployment => write!(f, "deployment"),
            ResourceType::Pod => write!(f, "pod"),
            ResourceType::Service => write!(f, "service"),
        }
    }
}

impl FromStr for ResourceType {
    type Err = ParserError;

    fn from_str(input: &str) -> Result<ResourceType, Self::Err> {
        match input {
            "deployment" => Ok(ResourceType::Deployment),
            "pod" => Ok(ResourceType::Pod),
            "service" => Ok(ResourceType::Service),
            _ => Err(ParserError::Unknown(format!(
                "Unexpected ResourceType for {}",
                input
            ))),
        }
    }
}

pub(crate) fn parse_sql(sql: &str) -> Result<ApiQueries, ParserError> {
    let dialect = GenericDialect {};

    // `-` is an incorrect char for SQL Queries, so we need to replace with another char
    // We will undo this replace during parsing stage
    let sql_replace = sql.replace('-', "_");

    // Parse the given SQL to AST
    let mut ast = Parser::parse_sql(&dialect, &sql_replace).unwrap();

    let query = match ast.pop().unwrap() {
        Statement::Query(query) => query,
        _ => {
            return Err(ParserError::Unsupported(
                "Only QUERY statements are supported!".to_string(),
            ));
        }
    };

    let mut queries = ApiQueries {
        namespaces: vec![],
        contexts: vec![],
        queries: vec![],
    };

    match &*query.body {
        SetExpr::Select(s) => {
            if s.projection.is_empty() {
                return Err(ParserError::SelectProjectionsRequired);
            }

            // SELECT ...
            for p in &s.projection {
                match p {
                    SelectItem::UnnamedExpr(o) => {
                        queries.namespaces.push(o.to_string().replace('_', "-"));
                    }
                    SelectItem::ExprWithAlias { .. } => {
                        return Err(ParserError::Unsupported(
                            "SELECT statement does not support ExprWithAlias selector!".to_string(),
                        ))
                    }
                    SelectItem::QualifiedWildcard(_, _) => {
                        return Err(ParserError::Unsupported(
                            "SELECT statement does not support QualifiedWildcard selector!"
                                .to_string(),
                        ))
                    }
                    SelectItem::Wildcard(_) => {
                        return Err(ParserError::Unsupported(
                            "SELECT statement does not support Wildcard selector!".to_string(),
                        ))
                    }
                }
            }

            if s.from.is_empty() {
                return Err(ParserError::SelectFromRequired);
            }

            // FROM ...
            for f in &s.from {
                if !f.joins.is_empty() {
                    return Err(ParserError::Unsupported(
                        "FROM statement does not support Join!".to_string(),
                    ));
                }
                match &f.relation {
                    TableFactor::Table {
                        name,
                        alias,
                        args,
                        with_hints,
                        ..
                    } => {
                        if alias.is_some() {
                            return Err(ParserError::Unsupported(
                                "FROM statement does not support Table aliases!".to_string(),
                            ));
                        }

                        if let Some(args) = args {
                            if !args.is_empty() {
                                return Err(ParserError::Unsupported(
                                    "FROM statement does not support Table ARGS!".to_string(),
                                ));
                            }
                        }
                        if !with_hints.is_empty() {
                            return Err(ParserError::Unsupported(
                                "FROM statement does not support Table HINT!".to_string(),
                            ));
                        }
                        queries.contexts.push(name.to_string().replace('_', "-"));
                    }
                    TableFactor::Derived { .. } => {
                        return Err(ParserError::Unsupported(
                            "FROM statement does not support Derived!".to_string(),
                        ))
                    }
                    TableFactor::TableFunction { .. } => {
                        return Err(ParserError::Unsupported(
                            "FROM statement does not support TableFunction!".to_string(),
                        ))
                    }
                    TableFactor::NestedJoin { .. } => {
                        return Err(ParserError::Unsupported(
                            "FROM statement does not support NestedJoin!".to_string(),
                        ))
                    }
                    TableFactor::UNNEST { .. } => {
                        return Err(ParserError::Unsupported(
                            "FROM statement does not support UNNEST!".to_string(),
                        ))
                    }
                }
            }

            // WHERE
            if let Some(w) = &s.selection {
                let mut plan_context = planner::PlanContext::default();
                let plan = w.to_owned().plan(&mut plan_context).unwrap();
                match plan {
                    Value::Queries(q) => queries.queries = q,
                    Value::Query(q) => queries.queries.push(q),
                    _ => {
                        return Err(ParserError::Unsupported(format!(
                            "Unable to handle unsupported query plan: {:?}",
                            plan
                        )))
                    }
                }
            } else {
                return Err(ParserError::Unsupported(
                    "WHERE statement is required in order to set --field-selector!".to_string(),
                ));
            }
        }
        _ => {
            return Err(ParserError::Unsupported(format!(
                "An unsupported query body given: {:?}",
                query.body
            )))
        }
    }

    Ok(queries)
}

pub(crate) fn parse_kubeconfig() -> Result<Kubeconfig, ParserError> {
    kube::config::Kubeconfig::read().map_err(ParserError::KubeConfigError)
}
