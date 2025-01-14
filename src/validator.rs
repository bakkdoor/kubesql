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

use kube::config::Kubeconfig;
use thiserror::Error;

#[derive(Error, Debug)]
#[error("ValidationError")]
pub enum ValidationError {
    #[error("ValidationError: Context not found in your KUBECONFIG: {0:?}")]
    ContextNotFound(Vec<String>),
}

pub fn validate_contexts(kubeconfig: Kubeconfig, ctxs: &[String]) -> Result<(), ValidationError> {
    let not_found = ctxs
        .iter()
        .filter(|item| kubeconfig.contexts.iter().all(|s| &s.name != *item))
        .collect::<Vec<&String>>();

    if !not_found.is_empty() {
        return Err(ValidationError::ContextNotFound(
            not_found.iter().map(|x| (*x).clone()).collect(),
        ));
    }

    Ok(())
}
