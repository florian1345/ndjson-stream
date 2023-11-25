use crate::bytes::AsBytes;
use crate::config::NdjsonConfig;
use crate::engine::NdjsonEngine;

use serde::Deserialize;

use serde_json::error::Result as JsonResult;

/// Wraps an iterator of data blocks, i.e. types implementing [AsBytes], and offers an [Iterator]
/// implementation over parsed NDJSON-records according to [Deserialize]. See [from_iter] and
/// [from_iter_with_config] for more details.
pub struct NdjsonIter<T, I> {
    engine: NdjsonEngine<T>,
    bytes_iterator: I
}

impl<T, I> NdjsonIter<T, I> {

    /// Creates a new NDJSON-iterator wrapping the given `bytes_iterator` with default
    /// [NdjsonConfig].
    pub fn new(bytes_iterator: I) -> NdjsonIter<T, I> {
        NdjsonIter {
            engine: NdjsonEngine::new(),
            bytes_iterator
        }
    }

    /// Creates a new NDJSON-iterator wrapping the given `bytes_iterator` with the given
    /// [NdjsonConfig] to control its behavior. See [NdjsonConfig] for more details.
    pub fn with_config(bytes_iterator: I, config: NdjsonConfig) -> NdjsonIter<T, I> {
        NdjsonIter {
            engine: NdjsonEngine::with_config(config),
            bytes_iterator
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
        // TODO handle rest

        loop {
            if let Some(result) = self.engine.pop() {
                return Some(result);
            }

            self.engine.input(self.bytes_iterator.next()?);
        }
    }
}

/// Wraps an iterator of data blocks, i.e. types implementing [AsBytes], obtained by
/// [IntoIterator::into_iter] on `into_iter` and offers an [Iterator] implementation over parsed
/// NDJSON-records according to [Deserialize]. The parser is configured with the default
/// [NdjsonConfig].
///
/// Example:
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
/// Example:
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

#[cfg(test)]
mod tests {

    use super::*;

    use kernal::prelude::*;

    use std::iter;
    use crate::config::EmptyLineHandling;

    use crate::test_util::{SingleThenPanicIter, TestStruct};

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
        let config = NdjsonConfig::default().
            with_empty_line_handling(EmptyLineHandling::IgnoreEmpty);
        let mut ndjson_iter: NdjsonIter<TestStruct, _> = from_iter_with_config(iter, config);

        assert_that!(ndjson_iter.next()).to_value().is_ok();
        assert_that!(ndjson_iter.next()).is_none();
    }
}
