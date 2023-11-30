use serde_json::Error as JsonError;

use std::convert::Infallible;

use thiserror::Error;

/// The errors which can occur when using a fallible-input-interface, such as
/// [FallibleNdjsonIter](crate::driver::iter::FallibleNdjsonIter) or
/// [FallibleNdjsonStream](crate::driver::stream::FallibleNdjsonStream).
#[derive(Error, Debug)]
pub enum FallibleNdjsonError<E> {

    /// Reading the fallible input failed. The error returned by the input on trying to read is
    /// wrapped in this variant.
    #[error("error reading input: {0}")]
    InputError(E),

    /// Parsing a JSON-line failed. The [serde_json::Error] is wrapped in this variant.
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

/// Syntactic sugar for a [Result] with the given value type `V` and a [FallibleNdjsonError] whose
/// input error type is the given error type `E`.
pub type FallibleNdjsonResult<V, E> = Result<V, FallibleNdjsonError<E>>;
