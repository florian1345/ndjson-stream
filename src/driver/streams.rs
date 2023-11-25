use crate::engine::NdjsonEngine;

use futures::{ready, Stream};

use pin_project_lite::pin_project;

use serde_json::error::Result as JsonResult;

use serde::Deserialize;

use std::pin::Pin;
use std::task::{Context, Poll};

use crate::bytes::AsBytes;
use crate::config::NdjsonConfig;

pin_project! {
    /// Wraps a [Stream] of data blocks, i.e. types implementing [AsBytes], and offers a [Stream]
    /// implementation over parsed NDJSON-records according to [Deserialize]. See [from_stream] for
    /// more details.
    pub struct NdjsonStream<T, S> {
        engine: NdjsonEngine<T>,
        #[pin]
        bytes_stream: S
    }
}

impl<T, S> NdjsonStream<T, S> {

    /// Creates a new NDJSON-stream wrapping the given `bytes_stream` with default [NdjsonConfig].
    pub fn new(bytes_stream: S) -> NdjsonStream<T, S> {
        NdjsonStream {
            engine: NdjsonEngine::new(),
            bytes_stream
        }
    }

    /// Creates a new NDJSON-stream wrapping the given `bytes_stream` with the given [NdjsonConfig]
    /// to control its behavior. See [NdjsonConfig] for more details.
    pub fn with_config(bytes_stream: S, config: NdjsonConfig) -> NdjsonStream<T, S> {
        NdjsonStream {
            engine: NdjsonEngine::with_config(config),
            bytes_stream
        }
    }
}

impl<T, S> Stream for NdjsonStream<T, S>
where
    for<'deserialize> T: Deserialize<'deserialize>,
    S: Stream,
    S::Item: AsBytes
{
    type Item = JsonResult<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<JsonResult<T>>> {
        // TODO handle rest

        let mut this = self.project();

        loop {
            if let Some(result) = this.engine.pop() {
                return Poll::Ready(Some(result));
            }

            let bytes = ready!(this.bytes_stream.as_mut().poll_next(cx));

            match bytes {
                Some(bytes) => this.engine.input(bytes),
                None => return Poll::Ready(None)
            }
        }
    }
}

/// Wraps a [Stream] of data blocks, i.e. types implementing [AsBytes], and offers a [Stream]
/// implementation over parsed NDJSON-records according to [Deserialize]. The parser is configured
/// with the default [NdjsonConfig].
///
/// Example:
///
/// ```
/// use futures::stream::{self, StreamExt};
///
/// let data_blocks = vec![
///     "123\n",
///     "456\n789\n"
/// ];
///
/// let mut ndjson_stream = ndjson_stream::from_stream::<u32, _>(stream::iter(data_blocks));
///
/// tokio_test::block_on(async {
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(123))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(456))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(789))));
///     assert!(ndjson_stream.next().await.is_none());
/// });
/// ```
pub fn from_stream<T, S>(bytes_stream: S) -> NdjsonStream<T, S> {
    NdjsonStream::new(bytes_stream)
}

/// Wraps a [Stream] of data blocks, i.e. types implementing [AsBytes], and offers a [Stream]
/// implementation over parsed NDJSON-records according to [Deserialize]. The parser is configured
/// with the given [NdjsonConfig].
///
/// Example:
///
/// ```
/// use futures::stream::{self, StreamExt};
/// use ndjson_stream::config::{EmptyLineHandling, NdjsonConfig};
///
/// let data_blocks = vec![
///     "123\n",
///     "456\n   \n789\n"
/// ];
/// let config = NdjsonConfig::default().with_empty_line_handling(EmptyLineHandling::IgnoreBlank);
///
/// let mut ndjson_stream =
///     ndjson_stream::from_stream_with_config::<u32, _>(stream::iter(data_blocks), config);
///
/// tokio_test::block_on(async {
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(123))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(456))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(789))));
///     assert!(ndjson_stream.next().await.is_none());
/// });
/// ```
pub fn from_stream_with_config<T, S>(bytes_stream: S, config: NdjsonConfig) -> NdjsonStream<T, S> {
    NdjsonStream::with_config(bytes_stream, config)
}

#[cfg(test)]
mod tests {
    use std::pin::pin;
    use futures::{Stream, StreamExt};
    use futures::stream;

    use kernal::prelude::*;

    use tokio_test::assert_pending;
    use tokio_test::task;

    use crate::bytes::AsBytes;
    use crate::config::EmptyLineHandling;
    use crate::test_util::{SingleThenPanicIter, TestStruct};

    use super::*;

    async fn collect<S>(bytes_stream: S) -> Vec<JsonResult<TestStruct>>
    where
        S: Stream,
        S::Item: AsBytes
    {
        from_stream(bytes_stream).collect().await
    }

    #[test]
    fn pending_stream_results_in_pending_item() {
        let mut ndjson_stream = from_stream::<TestStruct, _>(stream::pending::<&str>());

        let mut next = task::spawn(ndjson_stream.next());

        assert_pending!(next.poll());
    }

    #[test]
    fn empty_stream_results_in_empty_results() {
        let collected = tokio_test::block_on(collect::<_>(stream::empty::<&[u8]>()));

        assert_that!(collected).is_empty();
    }

    #[test]
    fn singleton_iter_with_single_json_line() {
        let stream = stream::once(async { "{\"key\":1,\"value\":2}\n" });
        let collected = tokio_test::block_on(collect(stream));

        assert_that!(collected).satisfies_exactly_in_given_order(dyn_assertions!(
            |it| assert_that!(it).contains_value(TestStruct { key: 1, value: 2 })
        ));
    }

    #[test]
    fn multiple_iter_items_compose_single_json_line() {
        let stream = stream::iter(vec!["{\"key\"", ":12,", "\"value\"", ":34}\n"]);
        let collected = tokio_test::block_on(collect(stream));

        assert_that!(collected).satisfies_exactly_in_given_order(dyn_assertions!(
            |it| assert_that!(it).contains_value(TestStruct { key: 12, value: 34 })
        ));
    }

    #[test]
    fn wrapped_stream_not_queried_while_sufficient_data_remains() {
        let iter = SingleThenPanicIter {
            data: Some("{\"key\":0,\"value\":0}\n{\"key\":0,\"value\":0}\n".to_owned())
        };
        let mut ndjson_stream = from_stream::<TestStruct, _>(stream::iter(iter));

        assert_that!(tokio_test::block_on(ndjson_stream.next())).is_some();
        assert_that!(tokio_test::block_on(ndjson_stream.next())).is_some();
    }

    #[test]
    fn stream_with_parse_always_config_respects_config() {
        let stream = stream::once(async { "{\"key\":1,\"value\":2}\n\n" });
        let config = NdjsonConfig::default()
            .with_empty_line_handling(EmptyLineHandling::ParseAlways);
        let mut ndjson_stream = pin!(from_stream_with_config::<TestStruct, _>(stream, config));

        assert_that!(tokio_test::block_on(ndjson_stream.next())).to_value().is_ok();
        assert_that!(tokio_test::block_on(ndjson_stream.next())).to_value().is_err();
    }

    #[test]
    fn stream_with_ignore_empty_config_respects_config() {
        let stream = stream::once(async { "{\"key\":1,\"value\":2}\n\n" });
        let config = NdjsonConfig::default()
            .with_empty_line_handling(EmptyLineHandling::IgnoreEmpty);
        let mut ndjson_stream = pin!(from_stream_with_config::<TestStruct, _>(stream, config));

        assert_that!(tokio_test::block_on(ndjson_stream.next())).to_value().is_ok();
        assert_that!(tokio_test::block_on(ndjson_stream.next())).is_none();
    }
}
