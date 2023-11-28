use std::convert::Infallible;
use crate::engine::NdjsonEngine;

use futures::{ready, Stream};

use pin_project_lite::pin_project;

use serde_json::error::Result as JsonResult;

use serde::Deserialize;

use std::pin::Pin;
use std::task::{Context, Poll};

use crate::bytes::AsBytes;
use crate::config::NdjsonConfig;
use crate::fallible::{FallibleNdjsonError, FallibleNdjsonResult};

pin_project! {
    struct MapResultInfallible<S> {
        #[pin]
        inner: S
    }
}

impl<S> MapResultInfallible<S> {
    fn new(inner: S) -> MapResultInfallible<S> {
        MapResultInfallible {
            inner
        }
    }
}

impl<S> Stream for MapResultInfallible<S>
where
    S: Stream
{
    type Item = Result<S::Item, Infallible>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let res = ready!(this.inner.as_mut().poll_next(cx));
        Poll::Ready(res.map(Ok))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

pin_project! {
    /// Wraps a [Stream] of data blocks, i.e. types implementing [AsBytes], and offers a [Stream]
    /// implementation over parsed NDJSON-records according to [Deserialize]. See [from_stream] and
    /// [from_stream_with_config] for more details.
    pub struct NdjsonStream<T, S> {
        #[pin]
        inner: FallibleNdjsonStream<T, MapResultInfallible<S>>
    }
}

impl<T, S> NdjsonStream<T, S> {

    /// Creates a new NDJSON-stream wrapping the given `bytes_stream` with default [NdjsonConfig].
    pub fn new(bytes_stream: S) -> NdjsonStream<T, S> {
        let inner_bytes_stream = MapResultInfallible::new(bytes_stream);

        NdjsonStream {
            inner: FallibleNdjsonStream::new(inner_bytes_stream)
        }
    }

    /// Creates a new NDJSON-stream wrapping the given `bytes_stream` with the given [NdjsonConfig]
    /// to control its behavior. See [NdjsonConfig] for more details.
    pub fn with_config(bytes_stream: S, config: NdjsonConfig) -> NdjsonStream<T, S> {
        let inner_bytes_stream = MapResultInfallible::new(bytes_stream);

        NdjsonStream {
            inner: FallibleNdjsonStream::with_config(inner_bytes_stream, config)
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
        let mut this = self.project();
        let inner_next = ready!(this.inner.as_mut().poll_next(cx));
        let next = inner_next
            .map(|fallible_res| fallible_res.map_err(FallibleNdjsonError::unwrap_json_error));

        Poll::Ready(next)
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

pin_project! {
    /// Wraps a [Stream] of [Result]s of data blocks, i.e. types implementing [AsBytes], and offers
    /// a [Stream] mplementation over parsed NDJSON-records according to [Deserialize], forwarding
    /// potential errors returned by the wrapped iterator. See [from_fallible_stream] and
    /// [from_fallible_stream_with_config] for more details.
    pub struct FallibleNdjsonStream<T, S> {
        engine: NdjsonEngine<T>,
        #[pin]
        bytes_stream: S
    }
}

impl<T, S> FallibleNdjsonStream<T, S> {

    /// Creates a new fallible NDJSON-stream wrapping the given `bytes_stream` with default
    /// [NdjsonConfig].
    pub fn new(bytes_stream: S) -> FallibleNdjsonStream<T, S> {
        FallibleNdjsonStream {
            engine: NdjsonEngine::new(),
            bytes_stream
        }
    }

    /// Creates a new fallible NDJSON-stream wrapping the given `bytes_stream` with the given
    /// [NdjsonConfig] to control its behavior. See [NdjsonConfig] for more details.
    pub fn with_config(bytes_stream: S, config: NdjsonConfig) -> FallibleNdjsonStream<T, S> {
        FallibleNdjsonStream {
            engine: NdjsonEngine::with_config(config),
            bytes_stream
        }
    }
}

impl<T, S, B, E> Stream for FallibleNdjsonStream<T, S>
where
    for<'deserialize> T: Deserialize<'deserialize>,
    S: Stream<Item = Result<B, E>>,
    B: AsBytes
{
    type Item = FallibleNdjsonResult<T, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // TODO handle rest

        let mut this = self.project();

        loop {
            if let Some(result) = this.engine.pop() {
                return match result {
                    Ok(value) => Poll::Ready(Some(Ok(value))),
                    Err(error) => Poll::Ready(Some(Err(FallibleNdjsonError::JsonError(error))))
                }
            }

            let bytes = ready!(this.bytes_stream.as_mut().poll_next(cx));

            match bytes {
                Some(Ok(bytes)) => this.engine.input(bytes),
                Some(Err(error)) =>
                    return Poll::Ready(Some(Err(FallibleNdjsonError::InputError(error)))),
                None => return Poll::Ready(None)
            }
        }
    }
}

/// Wraps a [Stream] of [Result]s of data blocks, i.e. types implementing [AsBytes], and offers a
/// [Stream] implementation over parsed NDJSON-records according to [Deserialize]. Errors in the
/// wrapped iterator are forwarded via [FallibleNdjsonError::InputError] , while parsing errors are
/// indicated via [FallibleNdjsonError::JsonError]. The parser is configured with the default
/// [NdjsonConfig].
///
/// Example:
///
/// ```
/// use futures::stream::{self, StreamExt};
/// use ndjson_stream::fallible::FallibleNdjsonError;
///
/// let data_block_results = vec![
///     Ok("123\n"),
///     Err("some error"),
///     Ok("456\n789\n")
/// ];
/// let data_stream = stream::iter(data_block_results);
///
/// let mut ndjson_stream = ndjson_stream::from_fallible_stream::<u32, _>(data_stream);
///
/// tokio_test::block_on(async {
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(123))));
///     assert!(matches!(ndjson_stream.next().await,
///         Some(Err(FallibleNdjsonError::InputError("some error")))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(456))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(789))));
///     assert!(ndjson_stream.next().await.is_none());
/// });
/// ```
pub fn from_fallible_stream<T, S>(bytes_stream: S) -> FallibleNdjsonStream<T, S> {
    FallibleNdjsonStream::new(bytes_stream)
}

/// Wraps a [Stream] of [Result]s of data blocks, i.e. types implementing [AsBytes], and offers a
/// [Stream] implementation over parsed NDJSON-records according to [Deserialize]. Errors in the
/// wrapped iterator are forwarded via [FallibleNdjsonError::InputError], while parsing errors are
/// indicated via [FallibleNdjsonError::JsonError]. The parser is configured with the given
/// [NdjsonConfig].
///
/// Example:
///
/// ```
/// use futures::stream::{self, StreamExt};
/// use ndjson_stream::config::{EmptyLineHandling, NdjsonConfig};
/// use ndjson_stream::fallible::FallibleNdjsonError;
///
/// let data_block_results = vec![
///     Ok("123\n"),
///     Err("some error"),
///     Ok("456\n   \n789\n")
/// ];
/// let data_stream = stream::iter(data_block_results);
/// let config = NdjsonConfig::default().with_empty_line_handling(EmptyLineHandling::IgnoreBlank);
///
/// let mut ndjson_stream =
///     ndjson_stream::from_fallible_stream_with_config::<u32, _>(data_stream, config);
///
/// tokio_test::block_on(async {
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(123))));
///     assert!(matches!(ndjson_stream.next().await,
///         Some(Err(FallibleNdjsonError::InputError("some error")))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(456))));
///     assert!(matches!(ndjson_stream.next().await, Some(Ok(789))));
///     assert!(ndjson_stream.next().await.is_none());
/// });
/// ```
pub fn from_fallible_stream_with_config<T, S>(bytes_stream: S, config: NdjsonConfig)
        -> FallibleNdjsonStream<T, S> {
    FallibleNdjsonStream::with_config(bytes_stream, config)
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
    use crate::test_util::{FallibleNdjsonResultAssertions, SingleThenPanicIter, TestStruct};

    use super::*;

    async fn collect<S>(bytes_stream: S) -> Vec<JsonResult<TestStruct>>
    where
        S: Stream,
        S::Item: AsBytes
    {
        from_stream(bytes_stream).collect().await
    }

    trait NextBlocking : Stream {
        fn next_blocking(&mut self) -> Option<Self::Item>;
    }

    impl<S: Stream + Unpin> NextBlocking for S {
        fn next_blocking(&mut self) -> Option<Self::Item> {
            tokio_test::block_on(self.next())
        }
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

        assert_that!(ndjson_stream.next_blocking()).is_some();
        assert_that!(ndjson_stream.next_blocking()).is_some();
    }

    #[test]
    fn stream_with_parse_always_config_respects_config() {
        let stream = stream::once(async { "{\"key\":1,\"value\":2}\n\n" });
        let config = NdjsonConfig::default()
            .with_empty_line_handling(EmptyLineHandling::ParseAlways);
        let mut ndjson_stream = pin!(from_stream_with_config::<TestStruct, _>(stream, config));

        assert_that!(ndjson_stream.next_blocking()).to_value().is_ok();
        assert_that!(ndjson_stream.next_blocking()).to_value().is_err();
    }

    #[test]
    fn stream_with_ignore_empty_config_respects_config() {
        let stream = stream::once(async { "{\"key\":1,\"value\":2}\n\n" });
        let config = NdjsonConfig::default()
            .with_empty_line_handling(EmptyLineHandling::IgnoreEmpty);
        let mut ndjson_stream = pin!(from_stream_with_config::<TestStruct, _>(stream, config));

        assert_that!(ndjson_stream.next_blocking()).to_value().is_ok();
        assert_that!(ndjson_stream.next_blocking()).is_none();
    }

    #[test]
    fn fallible_stream_correctly_forwards_json_error() {
        let stream = stream::once(async { Ok::<&str, &str>("\n") });
        let mut fallible_ndjson_stream = pin!(from_fallible_stream::<TestStruct, _>(stream));

        assert_that!(fallible_ndjson_stream.next_blocking()).to_value().is_json_error();
    }

    #[test]
    fn fallible_stream_correctly_forwards_input_error() {
        let stream = stream::once(async { Err::<&str, &str>("test message") });
        let mut fallible_ndjson_stream = pin!(from_fallible_stream::<TestStruct, _>(stream));

        assert_that!(fallible_ndjson_stream.next_blocking())
            .to_value()
            .is_input_error("test message");
    }

    #[test]
    fn fallible_stream_operates_correctly_with_interspersed_errors() {
        let data_vec = vec![
            Err("test message 1"),
            Ok("invalid json\n{\"key\":11,\"val"),
            Ok("ue\":22}\n{\"key\":33,\"value\":44}\ninvalid json\n"),
            Err("test message 2"),
            Ok("{\"key\":55,\"value\":66}\n")
        ];
        let data_stream = stream::iter(data_vec);
        let fallible_ndjson_stream = from_fallible_stream::<TestStruct, _>(data_stream);

        assert_that!(tokio_test::block_on(fallible_ndjson_stream.collect::<Vec<_>>()))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).is_input_error("test message 1"),
                |it| assert_that!(it).is_json_error(),
                |it| assert_that!(it).contains_value(TestStruct { key: 11, value: 22 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 33, value: 44 }),
                |it| assert_that!(it).is_json_error(),
                |it| assert_that!(it).is_input_error("test message 2"),
                |it| assert_that!(it).contains_value(TestStruct { key: 55, value: 66 })
            ));
    }
}
