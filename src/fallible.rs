use std::convert::Infallible;
use serde_json::Error as JsonError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum FallibleNdjsonError<E> {

    #[error("error reading input: {0}")]
    InputError(E),

    #[error("error parsing line: {0}")]
    JsonError(JsonError)
}

// TODO replace with never-type once available (https://github.com/rust-lang/rust/issues/35121)

impl FallibleNdjsonError<Infallible> {
    pub(crate) fn unwrap_json_error(self) -> JsonError {
        match self {
            FallibleNdjsonError::JsonError(err) => err,
            FallibleNdjsonError::InputError(err) => match err { }
        }
    }
}

pub type FallibleNdjsonResult<V, E> = Result<V, FallibleNdjsonError<E>>;
