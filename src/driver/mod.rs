#[cfg(feature = "iter")]
pub(crate) mod iter;

#[cfg(feature = "iter")]
pub use crate::driver::iter::NdjsonIter;

