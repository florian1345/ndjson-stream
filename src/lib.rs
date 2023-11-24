pub mod bytes;
pub mod driver;
pub mod engine;
pub mod config;

#[cfg(feature = "iter")]
pub use crate::driver::iter::from_iter;

#[cfg(feature = "streams")]
pub use crate::driver::streams::from_stream;

#[cfg(test)]
pub(crate) mod test_util {

    use serde::Deserialize;

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
}
