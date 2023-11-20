pub mod bytes;
pub mod driver;
pub mod engine;

pub use crate::driver::iter::from_iter;

#[cfg(test)]
pub(crate) mod test_struct {

    use serde::Deserialize;

    #[derive(Debug, Deserialize, Eq, PartialEq)]
    pub(crate) struct TestStruct {
        pub(crate) key: u64,
        pub(crate) value: u64
    }
}
