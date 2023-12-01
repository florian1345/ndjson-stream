use crate::as_bytes::AsBytes;
use crate::config::NdjsonConfig;
use crate::engine::NdjsonEngine;
use crate::fallible::{FallibleNdjsonError, FallibleNdjsonResult};

use std::convert::Infallible;
use std::iter::Fuse;

use serde::Deserialize;

use serde_json::error::Result as JsonResult;

struct MapResultInfallible<I> {
    inner: I
}

impl<I> MapResultInfallible<I> {
    fn new(inner: I) -> MapResultInfallible<I> {
        MapResultInfallible {
            inner
        }
    }
}

impl<I> Iterator for MapResultInfallible<I>
where
    I: Iterator
{
    type Item = Result<I::Item, Infallible>;

    fn next(&mut self) -> Option<Result<I::Item, Infallible>> {
        self.inner.next().map(Ok)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// Wraps an iterator of data blocks, i.e. types implementing [AsBytes], and offers an [Iterator]
/// implementation over parsed NDJSON-records according to [Deserialize]. See [from_iter] and
/// [from_iter_with_config] for more details.
pub struct NdjsonIter<T, I> {
    inner: FallibleNdjsonIter<T, MapResultInfallible<I>>
}

impl<T, I> NdjsonIter<T, I>
where
    I: Iterator
{

    /// Creates a new NDJSON-iterator wrapping the given `bytes_iterator` with default
    /// [NdjsonConfig].
    pub fn new(bytes_iterator: I) -> NdjsonIter<T, I> {
        let inner_bytes_iterator = MapResultInfallible::new(bytes_iterator);

        NdjsonIter {
            inner: FallibleNdjsonIter::new(inner_bytes_iterator)
        }
    }

    /// Creates a new NDJSON-iterator wrapping the given `bytes_iterator` with the given
    /// [NdjsonConfig] to control its behavior. See [NdjsonConfig] for more details.
    pub fn with_config(bytes_iterator: I, config: NdjsonConfig) -> NdjsonIter<T, I> {
        let inner_bytes_iterator = MapResultInfallible::new(bytes_iterator);

        NdjsonIter {
            inner: FallibleNdjsonIter::with_config(inner_bytes_iterator, config)
        }
    }
}

impl<T, I> Iterator for NdjsonIter<T, I>
where
    for<'deserialize> T: Deserialize<'deserialize>,
    I: Iterator,
    I::Item: AsBytes
{
    type Item = JsonResult<T>;

    fn next(&mut self) -> Option<JsonResult<T>> {
        Some(self.inner.next()?.map_err(FallibleNdjsonError::unwrap_json_error))
    }
}

/// Wraps an iterator of data blocks, i.e. types implementing [AsBytes], obtained by
/// [IntoIterator::into_iter] on `into_iter` and offers an [Iterator] implementation over parsed
/// NDJSON-records according to [Deserialize]. The parser is configured with the default
/// [NdjsonConfig].
///
/// # Example
///
/// ```
/// let data_blocks = vec![
///     "123\n",
///     "456\n789\n"
/// ];
///
/// let mut ndjson_iter = ndjson_stream::from_iter::<u32, _>(data_blocks);
///
/// assert!(matches!(ndjson_iter.next(), Some(Ok(123))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(456))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(789))));
/// assert!(ndjson_iter.next().is_none());
/// ```
pub fn from_iter<T, I>(into_iter: I) -> NdjsonIter<T, I::IntoIter>
where
    I: IntoIterator
{
    NdjsonIter::new(into_iter.into_iter())
}

/// Wraps an iterator of data blocks, i.e. types implementing [AsBytes], obtained by
/// [IntoIterator::into_iter] on `into_iter` and offers an [Iterator] implementation over parsed
/// NDJSON-records according to [Deserialize]. The parser is configured with the given
/// [NdjsonConfig].
///
/// # Example
///
/// ```
/// use ndjson_stream::config::{EmptyLineHandling, NdjsonConfig};
///
/// let data_blocks = vec![
///     "123\n",
///     "456\n   \n789\n"
/// ];
/// let config = NdjsonConfig::default().with_empty_line_handling(EmptyLineHandling::IgnoreBlank);
///
/// let mut ndjson_iter = ndjson_stream::from_iter_with_config::<u32, _>(data_blocks, config);
///
/// assert!(matches!(ndjson_iter.next(), Some(Ok(123))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(456))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(789))));
/// assert!(ndjson_iter.next().is_none());
/// ```
pub fn from_iter_with_config<T, I>(into_iter: I, config: NdjsonConfig) -> NdjsonIter<T, I::IntoIter>
where
    I: IntoIterator
{
    NdjsonIter::with_config(into_iter.into_iter(), config)
}

/// Wraps an iterator over [Result]s of data blocks, i.e. types implementing [AsBytes], and offers
/// an [Iterator] implementation over parsed NDJSON-records according to [Deserialize], forwarding
/// potential errors returned by the wrapped iterator. See [from_fallible_iter] and
/// [from_fallible_iter_with_config] for more details.
pub struct FallibleNdjsonIter<T, I> {
    engine: NdjsonEngine<T>,
    bytes_iterator: Fuse<I>
}

impl<T, I> FallibleNdjsonIter<T, I>
where
    I: Iterator
{

    /// Creates a new fallible NDJSON-iterator wrapping the given `bytes_iterator` with default
    /// [NdjsonConfig].
    pub fn new(bytes_iterator: I) -> FallibleNdjsonIter<T, I> {
        FallibleNdjsonIter {
            engine: NdjsonEngine::new(),
            bytes_iterator: bytes_iterator.fuse()
        }
    }

    /// Creates a new fallible NDJSON-iterator wrapping the given `bytes_iterator` with the given
    /// [NdjsonConfig] to control its behavior. See [NdjsonConfig] for more details.
    pub fn with_config(bytes_iterator: I, config: NdjsonConfig) -> FallibleNdjsonIter<T, I> {
        FallibleNdjsonIter {
            engine: NdjsonEngine::with_config(config),
            bytes_iterator: bytes_iterator.fuse()
        }
    }
}

impl<T, I, B, E> Iterator for FallibleNdjsonIter<T, I>
where
    for<'deserialize> T: Deserialize<'deserialize>,
    I: Iterator<Item = Result<B, E>>,
    B: AsBytes
{
    type Item = FallibleNdjsonResult<T, E>;

    fn next(&mut self) -> Option<FallibleNdjsonResult<T, E>> {
        loop {
            if let Some(result) = self.engine.pop() {
                return match result {
                    Ok(value) => Some(Ok(value)),
                    Err(error) => Some(Err(FallibleNdjsonError::JsonError(error)))
                }
            }

            match self.bytes_iterator.next() {
                Some(Ok(bytes)) => self.engine.input(bytes),
                Some(Err(error)) => return Some(Err(FallibleNdjsonError::InputError(error))),
                None => {
                    self.engine.finalize();
                    return self.engine.pop()
                        .map(|res| res.map_err(FallibleNdjsonError::JsonError));
                }
            }
        }
    }
}

/// Wraps an iterator of [Result]s of data blocks, i.e. types implementing [AsBytes], obtained by
/// [IntoIterator::into_iter] on `into_iter` and offers an [Iterator] implementation over parsed
/// NDJSON-records according to [Deserialize]. Errors in the wrapped iterator are forwarded via
/// [FallibleNdjsonError::InputError], while parsing errors are indicated via
/// [FallibleNdjsonError::JsonError]. The parser is configured with the default [NdjsonConfig].
///
/// # Example
///
/// ```
/// use ndjson_stream::fallible::FallibleNdjsonError;
///
/// let data_block_results = vec![
///     Ok("123\n"),
///     Err("some error"),
///     Ok("456\n789\n")
/// ];
///
/// let mut ndjson_iter = ndjson_stream::from_fallible_iter::<u32, _>(data_block_results);
///
/// assert!(matches!(ndjson_iter.next(), Some(Ok(123))));
/// assert!(matches!(ndjson_iter.next(), Some(Err(FallibleNdjsonError::InputError("some error")))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(456))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(789))));
/// assert!(ndjson_iter.next().is_none());
/// ```
pub fn from_fallible_iter<T, I>(into_iter: I) -> FallibleNdjsonIter<T, I::IntoIter>
where
    I: IntoIterator
{
    FallibleNdjsonIter::new(into_iter.into_iter())
}

/// Wraps an iterator of [Result]s of data blocks, i.e. types implementing [AsBytes], obtained by
/// [IntoIterator::into_iter] on `into_iter` and offers an [Iterator] implementation over parsed
/// NDJSON-records according to [Deserialize]. Errors in the wrapped iterator are forwarded via
/// [FallibleNdjsonError::InputError], while parsing errors are indicated via
/// [FallibleNdjsonError::JsonError]. The parser is configured with the given [NdjsonConfig].
///
/// # Example
///
/// ```
/// use ndjson_stream::config::{EmptyLineHandling, NdjsonConfig};
/// use ndjson_stream::fallible::FallibleNdjsonError;
///
/// let data_block_results = vec![
///     Ok("123\n"),
///     Err("some error"),
///     Ok("456\n   \n789\n")
/// ];
/// let config = NdjsonConfig::default().with_empty_line_handling(EmptyLineHandling::IgnoreBlank);
///
/// let mut ndjson_iter =
///     ndjson_stream::from_fallible_iter_with_config::<u32, _>(data_block_results, config);
///
/// assert!(matches!(ndjson_iter.next(), Some(Ok(123))));
/// assert!(matches!(ndjson_iter.next(), Some(Err(FallibleNdjsonError::InputError("some error")))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(456))));
/// assert!(matches!(ndjson_iter.next(), Some(Ok(789))));
/// assert!(ndjson_iter.next().is_none());
/// ```
pub fn from_fallible_iter_with_config<T, I>(into_iter: I, config: NdjsonConfig)
    -> FallibleNdjsonIter<T, I::IntoIter>
where
    I: IntoIterator
{
    FallibleNdjsonIter::with_config(into_iter.into_iter(), config)
}

#[cfg(test)]
mod tests {

    use super::*;

    use kernal::prelude::*;

    use std::iter;

    use crate::config::EmptyLineHandling;
    use crate::test_util::{FallibleNdjsonResultAssertions, SingleThenPanicIter, TestStruct};

    fn collect<I>(into_iter: I) -> Vec<JsonResult<TestStruct>>
    where
        I: IntoIterator,
        I::Item: AsBytes
    {
        from_iter(into_iter).collect::<Vec<_>>()
    }

    #[test]
    fn empty_iter_results_in_empty_results() {
        assert_that!(collect::<_>(iter::empty::<&[u8]>())).is_empty();
    }

    #[test]
    fn singleton_iter_with_single_json_line() {
        let iter = iter::once("{\"key\":1,\"value\":2}\n");

        assert_that!(collect(iter)).satisfies_exactly_in_given_order(dyn_assertions!(
            |it| assert_that!(it).contains_value(TestStruct { key: 1, value: 2 })
        ));
    }

    #[test]
    fn multiple_iter_items_compose_single_json_line() {
        let vec = vec!["{\"key\"", ":12,", "\"value\"", ":34}\n"];

        assert_that!(collect(vec)).satisfies_exactly_in_given_order(dyn_assertions!(
            |it| assert_that!(it).contains_value(TestStruct { key: 12, value: 34 })
        ));
    }

    #[test]
    fn wrapped_iter_not_queried_while_sufficient_data_remains() {
        let iter = SingleThenPanicIter {
            data: Some("{\"key\":1,\"value\":2}\n{\"key\":3,\"value\":4}\n".to_owned())
        };
        let mut ndjson_iter = NdjsonIter::<TestStruct, _>::new(iter);

        assert_that!(ndjson_iter.next()).is_some();
        assert_that!(ndjson_iter.next()).is_some();
    }

    #[test]
    fn iter_with_parse_always_config_respects_config() {
        let iter = iter::once("{\"key\":1,\"value\":2}\n\n");
        let config = NdjsonConfig::default()
            .with_empty_line_handling(EmptyLineHandling::ParseAlways);
        let mut ndjson_iter: NdjsonIter<TestStruct, _> = from_iter_with_config(iter, config);

        assert_that!(ndjson_iter.next()).to_value().is_ok();
        assert_that!(ndjson_iter.next()).to_value().is_err();
    }

    #[test]
    fn iter_with_ignore_empty_config_respects_config() {
        let iter = iter::once("{\"key\":1,\"value\":2}\n\n");
        let config = NdjsonConfig::default()
            .with_empty_line_handling(EmptyLineHandling::IgnoreEmpty);
        let mut ndjson_iter: NdjsonIter<TestStruct, _> = from_iter_with_config(iter, config);

        assert_that!(ndjson_iter.next()).to_value().is_ok();
        assert_that!(ndjson_iter.next()).is_none();
    }

    #[test]
    fn iter_with_parse_rest_handles_valid_finalization() {
        let iter = iter::once("{\"key\":1,\"value\":2}");
        let config = NdjsonConfig::default().with_parse_rest(true);
        let mut ndjson_iter: NdjsonIter<TestStruct, _> = from_iter_with_config(iter, config);

        assert_that!(ndjson_iter.next()).to_value().contains_value(TestStruct { key: 1, value: 2 });
        assert_that!(ndjson_iter.next()).is_none();
    }

    #[test]
    fn iter_with_parse_rest_handles_invalid_finalization() {
        let iter = iter::once("{\"key\":1,");
        let config = NdjsonConfig::default().with_parse_rest(true);
        let mut ndjson_iter: NdjsonIter<TestStruct, _> = from_iter_with_config(iter, config);

        assert_that!(ndjson_iter.next()).to_value().is_err();
        assert_that!(ndjson_iter.next()).is_none();
    }

    #[test]
    fn iter_without_parse_rest_does_not_handle_finalization() {
        let iter = iter::once("some text");
        let config = NdjsonConfig::default().with_parse_rest(false);
        let mut ndjson_iter: NdjsonIter<TestStruct, _> = from_iter_with_config(iter, config);

        assert_that!(ndjson_iter.next()).is_none();
    }

    #[test]
    fn iter_fuses_bytes_iter() {
        #[derive(Default)]
        struct NoneThenPanicIter {
            returned_none: bool
        }

        impl Iterator for NoneThenPanicIter {
            type Item = Vec<u8>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.returned_none {
                    panic!("iterator queried twice");
                }

                self.returned_none = true;
                None
            }
        }

        let iter = NoneThenPanicIter::default();
        let config = NdjsonConfig::default().with_parse_rest(true);
        let mut ndjson_iter: NdjsonIter<TestStruct, _> = from_iter_with_config(iter, config);

        assert_that!(ndjson_iter.next()).is_none();
        assert_that!(ndjson_iter.next()).is_none();
    }

    #[test]
    fn fallible_iter_correctly_forwards_json_error() {
        let iter = iter::once::<Result<&str, &str>>(Ok("\n"));
        let mut fallible_ndjson_iter: FallibleNdjsonIter<TestStruct, _> = from_fallible_iter(iter);

        assert_that!(fallible_ndjson_iter.next()).to_value().is_json_error();
    }

    #[test]
    fn fallible_iter_correctly_forwards_input_error() {
        let iter = iter::once::<Result<&str, &str>>(Err("test message"));
        let mut fallible_ndjson_iter: FallibleNdjsonIter<TestStruct, _> = from_fallible_iter(iter);

        assert_that!(fallible_ndjson_iter.next()).to_value().is_input_error("test message");
    }

    #[test]
    fn fallible_iter_operates_correctly_with_interspersed_errors() {
        let data_vec = vec![
            Ok("{\"key\":42,\"val"),
            Err("test message 1"),
            Ok("ue\":24}\n{\"key\":21,\"value\":12}\ninvalid json\n"),
            Err("test message 2"),
            Ok("{\"key\":63,\"value\":36}\n")
        ];
        let fallible_ndjson_iter: FallibleNdjsonIter<TestStruct, _> =
            from_fallible_iter(data_vec);

        assert_that!(fallible_ndjson_iter.collect::<Vec<_>>())
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).is_input_error("test message 1"),
                |it| assert_that!(it).contains_value(TestStruct { key: 42, value: 24 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 21, value: 12 }),
                |it| assert_that!(it).is_json_error(),
                |it| assert_that!(it).is_input_error("test message 2"),
                |it| assert_that!(it).contains_value(TestStruct { key: 63, value: 36 })
            ));
    }
}
