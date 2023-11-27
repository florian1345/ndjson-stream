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

#[cfg(feature = "streams")]
pub use crate::driver::streams::from_stream;

#[cfg(feature = "streams")]
pub use crate::driver::streams::from_stream_with_config;

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
