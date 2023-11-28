#[cfg(feature = "iter")]
pub(crate) mod iter;

#[cfg(feature = "streams")]
pub(crate) mod streams;

#[cfg(feature = "iter")]
pub use crate::driver::iter::NdjsonIter;

#[cfg(feature = "iter")]
pub use crate::driver::iter::FallibleNdjsonIter;

#[cfg(feature = "streams")]
pub use crate::driver::streams::NdjsonStream;

#[cfg(feature = "streams")]
pub use crate::driver::streams::FallibleNdjsonStream;
