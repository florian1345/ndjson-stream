//! This module defines the [AsBytes] with baseline implementations.

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "bytes")]
use bytes::{Bytes, BytesMut};

/// A trait for types which represent a contiguous block of bytes, such as `&[u8]` or `Vec<u8>`.
pub trait AsBytes {

    /// Gets a slice of the entire block of bytes contained in this instance.
    fn as_bytes(&self) -> &[u8];
}

impl AsBytes for [u8] {
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl<const LEN: usize> AsBytes for [u8; LEN] {
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl AsBytes for Vec<u8> {
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl AsBytes for str {
    fn as_bytes(&self) -> &[u8] {
        str::as_bytes(self)
    }
}

impl AsBytes for String {
    fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

#[cfg(feature = "bytes")]
impl AsBytes for Bytes {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }
}

#[cfg(feature = "bytes")]
impl AsBytes for BytesMut {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }
}

impl<T: AsBytes + ?Sized> AsBytes for &T {
    fn as_bytes(&self) -> &[u8] {
        T::as_bytes(self)
    }
}

impl<T: AsBytes + ?Sized> AsBytes for &mut T {
    fn as_bytes(&self) -> &[u8] {
        T::as_bytes(self)
    }
}

impl<T: AsBytes + ?Sized> AsBytes for Box<T> {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }
}

impl<'cow, T: AsBytes + Clone + ?Sized> AsBytes for Cow<'cow, T> {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }
}

impl<T: AsBytes + ?Sized> AsBytes for Rc<T> {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }
}

impl<T: AsBytes + ?Sized> AsBytes for Arc<T> {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref().as_bytes()
    }
}

#[cfg(all(test, feature = "bytes"))]
mod bytes_tests {

    use bytes::Bytes;
    use kernal::prelude::*;

    use super::*;

    #[test]
    fn bytes_works() {
        let bytes = Bytes::from(&[1, 2, 3][..]);

        assert_that!(bytes.as_bytes()).contains_exactly_in_given_order([1, 2, 3]);
    }

    #[test]
    fn bytes_mut_works() {
        let bytes_mut = BytesMut::from(&[3, 2, 1][..]);

        assert_that!(bytes_mut.as_bytes()).contains_exactly_in_given_order([3, 2, 1]);
    }
}
