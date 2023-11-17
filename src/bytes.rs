use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

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
