//! `ndjson-stream` offers a variety of NDJSON-parsers which accept data in chunks and process these
//! chunks before reading further, thus enabling a streaming-style use. The crate offers a low-level
//! interface in the [engine] module and more high-level interfaces for synchronous and asynchronous
//! NDJSON processing, which are available at the crate root (see for example [from_iter]). The
//! parser accepts any input which implements the [AsBytes](bytes::AsBytes) trait, which are the
//! most common data containers in core Rust and the standard library (e.g. `Vec<u8>` or `&str`).
//!
//! `ndjson-stream` uses the [serde] crate to parse individual lines. Hence, the output type of the
//! parser must implement [Deserialize](serde::Deserialize).
//!
//! # High-level example
//!
//! As an example, we will look at the iterator interface. The most basic form can be instantiated
//! with [from_iter]. We have to provide an iterator over data blocks, implementing
//! [AsBytes](bytes::AsBytes), and obtain an iterator over parsed NDJSON-records. Actually, the
//! exact return type is a `Result` which may contain a JSON-error in case a line is not valid JSON
//! or does not match the schema of the output type.
//!
//! The example below demonstrates both the happy-path as well as parsing errors.
//!
//! ```
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, Eq, PartialEq)]
//! struct Person {
//!     name: String,
//!     age: u16
//! }
//!
//! let data_blocks = vec![
//!     "{\"name\":\"Alice\",\"age\":25}\n",
//!     "{\"this\":\"is\",\"not\":\"valid\"}\n",
//!     "{\"name\":\"Bob\",",
//!     "\"age\":35}\r\n"
//! ];
//! let mut ndjson_iter = ndjson_stream::from_iter::<Person, _>(data_blocks);
//!
//! assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Alice".into(), age: 25 });
//! assert!(ndjson_iter.next().unwrap().is_err());
//! assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Bob".into(), age: 35 });
//! assert!(ndjson_iter.next().is_none());
//! ```
//!
//! # Configuration
//!
//! There are several configuration options available to control how the parser behaves in certain
//! situations. See [NdjsonConfig](config::NdjsonConfig) for more details. To specify the config
//! used for a parser, use the appropriate `_with_config`-suffixed function.
//!
//! In the example below, we use [from_iter_with_config] to construct an NDJSON-iterator which
//! ignores blank lines. That is, it does not produce an output record for any line which consists
//! only of whitespace rather than attempting to parse it and raising a JSON-error.
//!
//! ```
//! use ndjson_stream::config::{EmptyLineHandling, NdjsonConfig};
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, Eq, PartialEq)]
//! struct Person {
//!     name: String,
//!     age: u16
//! }
//!
//! let data_blocks = vec![
//!     "{\"name\":\"Charlie\",\"age\":32}\n",
//!     "   \n",
//!     "{\"name\":\"Dolores\",\"age\":41}\n"
//! ];
//! let config = NdjsonConfig::default().with_empty_line_handling(EmptyLineHandling::IgnoreBlank);
//! let mut ndjson_iter = ndjson_stream::from_iter_with_config::<Person, _>(data_blocks, config);
//!
//! assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Charlie".into(), age: 32 });
//! assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Dolores".into(), age: 41 });
//! assert!(ndjson_iter.next().is_none());
//! ```
//!
//! # Fallibility
//!
//! In addition to the ordinary interfaces, there is a fallible counterpart for each one. "Fallible"
//! in this context refers to the input data source - in the examples above the iterator of
//! `data_blocks`.
//!
//! Fallible parsers accept as input a data source which returns [Result]s with some error type and
//! forward potential read errors to the user. See
//! [FallibleNdjsonError](fallible::FallibleNdjsonError) for more details on how the error is
//! communicated.
//!
//! In the example below, we use a fallible iterator interface.
//!
//! ```
//! use ndjson_stream::fallible::FallibleNdjsonError;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, Eq, PartialEq)]
//! struct Person {
//!     name: String,
//!     age: u16
//! }
//!
//! let data_blocks = vec![
//!     Ok("{\"name\":\"Eve\",\"age\":22}\n"),
//!     Err("error"),
//!     Ok("{\"invalid\":json}\n")
//! ];
//! let mut ndjson_iter = ndjson_stream::from_fallible_iter::<Person, _>(data_blocks);
//!
//! assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Eve".into(), age: 22 });
//! assert!(matches!(ndjson_iter.next(), Some(Err(FallibleNdjsonError::InputError("error")))));
//! assert!(matches!(ndjson_iter.next(), Some(Err(FallibleNdjsonError::JsonError(_)))));
//! assert!(ndjson_iter.next().is_none());
//! ```
//!
//! # Crate features
//!
//! * `iter` (default): Enables the [Iterator]-style interface ([from_iter] family).
//! * `stream`: Enables the [Stream](futures::Stream)-style interface from the `futures` crate
//! ([from_stream] family).

pub mod bytes;
pub mod config;
pub mod driver;
pub mod engine;
pub mod fallible;

#[cfg(feature = "iter")]
pub use crate::driver::iter::from_iter;

#[cfg(feature = "iter")]
pub use crate::driver::iter::from_iter_with_config;

#[cfg(feature = "iter")]
pub use crate::driver::iter::from_fallible_iter;

#[cfg(feature = "iter")]
pub use crate::driver::iter::from_fallible_iter_with_config;

#[cfg(feature = "stream")]
pub use crate::driver::stream::from_stream;

#[cfg(feature = "stream")]
pub use crate::driver::stream::from_stream_with_config;

#[cfg(feature = "stream")]
pub use crate::driver::stream::from_fallible_stream;

#[cfg(feature = "stream")]
pub use crate::driver::stream::from_fallible_stream_with_config;

#[cfg(test)]
pub(crate) mod test_util {
    use std::borrow::Borrow;
    use std::fmt::Debug;

    use kernal::{AssertThat, AssertThatData, Failure};

    use serde::Deserialize;
    use crate::fallible::{FallibleNdjsonError, FallibleNdjsonResult};

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    pub(crate) struct TestStruct {
        pub(crate) key: u64,
        pub(crate) value: u64
    }

    pub(crate) struct SingleThenPanicIter {
        pub(crate) data: Option<String>
    }

    impl Iterator for SingleThenPanicIter {
        type Item = String;

        fn next(&mut self) -> Option<String> {
            Some(self.data.take().expect("iterator queried twice"))
        }
    }

    pub(crate) trait FallibleNdjsonResultAssertions<V, E> {

        fn is_json_error(self) -> Self;

        fn is_input_error(self, expected: impl Borrow<E>) -> Self;
    }

    impl<V, E, R> FallibleNdjsonResultAssertions<V, E> for AssertThat<R>
    where
        E: Debug + PartialEq,
        R: Borrow<FallibleNdjsonResult<V, E>>
    {
        fn is_json_error(self) -> Self {
            let failure_start = Failure::new(&self).expected_it("to contain a JSON-error");

            match self.data().borrow() {
                Err(FallibleNdjsonError::JsonError(_)) => self,
                Err(FallibleNdjsonError::InputError(_)) =>
                    failure_start.but_it("was an input error").fail(),
                Ok(_) => failure_start.but_it("was Ok").fail()
            }
        }

        fn is_input_error(self, expected: impl Borrow<E>) -> Self {
            let expected = expected.borrow();
            let failure_start = Failure::new(&self)
                .expected_it(format!("to contain the input error <{:?}>", expected));

            match self.data().borrow() {
                Err(FallibleNdjsonError::InputError(actual)) if actual == expected => self,
                Err(FallibleNdjsonError::InputError(actual)) =>
                    failure_start
                        .but_it(format!("contained the input error <{:?}>", actual))
                        .fail(),
                Err(FallibleNdjsonError::JsonError(_)) =>
                    failure_start.but_it("was a JSON-error").fail(),
                Ok(_) => failure_start.but_it("was Ok").fail(),
            }
        }
    }
}
