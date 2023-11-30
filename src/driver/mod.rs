//! This module contains the higher-level drivers of the NDJSON-parser. Convenience functions to
//! construct these are found at top-level of the crate.

#[cfg(feature = "iter")]
pub(crate) mod iter;

#[cfg(feature = "stream")]
pub(crate) mod stream;

#[cfg(feature = "iter")]
pub use crate::driver::iter::NdjsonIter;

#[cfg(feature = "iter")]
pub use crate::driver::iter::FallibleNdjsonIter;

#[cfg(feature = "stream")]
pub use crate::driver::stream::NdjsonStream;

#[cfg(feature = "stream")]
pub use crate::driver::stream::FallibleNdjsonStream;
