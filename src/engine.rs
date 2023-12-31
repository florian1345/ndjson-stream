//! This module contains the low-level NDJSON parsing logic in the form of the [NdjsonEngine]. You
//! should usually not have to use this directly, but rather access a higher-level interface such as
//! iterators.

use std::collections::VecDeque;
use std::str;

use serde::Deserialize;

use serde_json::error::Result as JsonResult;

use crate::as_bytes::AsBytes;
use crate::config::{EmptyLineHandling, NdjsonConfig};

fn index_of<T: Eq>(data: &[T], search: T) -> Option<usize> {
    data.iter().enumerate()
        .find(|&(_, item)| item == &search)
        .map(|(index, _)| index)
}

const NEW_LINE: u8 = b'\n';

/// The low-level engine parsing NDJSON-data given as byte slices into objects of the type parameter
/// `T`. Data is supplied in chunks and parsed objects can subsequently be read from a queue.
///
/// Users of this crate should usually not have to use this struct but rather a higher-level
/// interface such as iterators.
pub struct NdjsonEngine<T> {
    in_queue: Vec<u8>,
    out_queue: VecDeque<JsonResult<T>>,
    config: NdjsonConfig
}

impl<T> NdjsonEngine<T> {

    /// Creates a new NDJSON-engine for objects of the given type parameter with default
    /// [NdjsonConfig].
    pub fn new() -> NdjsonEngine<T> {
        NdjsonEngine::with_config(NdjsonConfig::default())
    }

    /// Creates a new NDJSON-engine for objects of the given type parameter with the given
    /// [NdjsonConfig] to control its behavior. See [NdjsonConfig] for more details.
    pub fn with_config(config: NdjsonConfig) -> NdjsonEngine<T> {
        NdjsonEngine {
            in_queue: Vec::new(),
            out_queue: VecDeque::new(),
            config
        }
    }

    /// Reads the next element from the queue of parsed items, if sufficient NDJSON-data has been
    /// supplied previously via [NdjsonEngine::input], that is, a newline character has been
    /// observed. If the input until the newline is not valid JSON, the parse error is returned. If
    /// no element is available in the queue, `None` is returned.
    pub fn pop(&mut self) -> Option<JsonResult<T>> {
        self.out_queue.pop_front()
    }
}

fn is_blank(string: &str) -> bool {
    string.chars().all(char::is_whitespace)
}

fn parse_line<T>(bytes: &[u8], empty_line_handling: EmptyLineHandling) -> Option<JsonResult<T>>
where
    for<'deserialize> T: Deserialize<'deserialize>
{
    let should_ignore = match empty_line_handling {
        EmptyLineHandling::ParseAlways => false,
        EmptyLineHandling::IgnoreEmpty => bytes.is_empty() || bytes == [b'\r'],
        EmptyLineHandling::IgnoreBlank => str::from_utf8(bytes).is_ok_and(is_blank)
    };

    if should_ignore {
        None
    }
    else {
        Some(serde_json::from_slice(bytes))
    }
}

impl<T> NdjsonEngine<T>
where
    for<'deserialize> T: Deserialize<'deserialize>
{

    /// Parses the given data as NDJSON. In case the end does not match up with a newline, the rest
    /// is stored in an internal cache. Consequently, the rest from a previous call to this method
    /// is prepended to the given data in case a newline is encountered.
    pub fn input(&mut self, data: impl AsBytes) {
        let mut data = data.as_bytes();

        while let Some(newline_idx) = index_of(data, NEW_LINE) {
            let data_until_split = &data[..newline_idx];

            let next_item_bytes = if self.in_queue.is_empty() {
                data_until_split
            }
            else {
                self.in_queue.extend_from_slice(data_until_split);
                &self.in_queue
            };

            if let Some(item) = parse_line(next_item_bytes, self.config.empty_line_handling) {
                self.out_queue.push_back(item);
            }

            self.in_queue.clear();
            data = &data[(newline_idx + 1)..];
        }

        self.in_queue.extend_from_slice(data);
    }

    /// Parses the rest leftover from previous calls to [NdjsonEngine::input], i.e. the data after
    /// the last given newline character, if all of the following conditions are met.
    ///
    /// * The engine uses a config with [NdjsonConfig::with_parse_rest] set to `true`.
    /// * There is non-empty data left to parse. In other words, the previous provided input did not
    /// end with a newline character.
    /// * The rest is not considered empty by the handling configured in
    /// [NdjsonConfig::with_empty_line_handling]. That is, if the rest consists only of whitespace
    /// and [EmptyLineHandling::IgnoreBlank] is used, the rest is not parsed.
    ///
    /// In any case, the rest is discarded from the input buffer. Therefore, this function is
    /// idempotent.
    ///
    /// Note: This function is intended to be called after the input ended, but there is no
    /// validation in place to check that [NdjsonEngine::input] is not called afterwards. Doing this
    /// anyway may lead to unexpected behavior, as JSON-lines may be partially discarded.
    pub fn finalize(&mut self) {
        if self.config.parse_rest {
            let empty_line_handling = match self.config.empty_line_handling {
                EmptyLineHandling::ParseAlways => EmptyLineHandling::IgnoreEmpty,
                empty_line_handling => empty_line_handling
            };

            if let Some(item) = parse_line(&self.in_queue, empty_line_handling) {
                self.out_queue.push_back(item);
            }
        }

        self.in_queue.clear();
    }
}

impl<T> Default for NdjsonEngine<T> {
    fn default() -> NdjsonEngine<T> {
        NdjsonEngine::new()
    }
}

#[cfg(test)]
mod tests {

    use kernal::prelude::*;

    use serde_json::error::Result as JsonResult;

    use std::borrow::Cow;
    use std::iter;
    use std::rc::Rc;
    use std::sync::Arc;
    use crate::config::{EmptyLineHandling, NdjsonConfig};

    use crate::engine::NdjsonEngine;
    use crate::test_util::TestStruct;

    fn collect_output(mut engine: NdjsonEngine<TestStruct>)
            -> Vec<JsonResult<TestStruct>> {
        iter::from_fn(|| engine.pop()).collect::<Vec<_>>()
    }

    #[test]
    fn no_input() {
        let engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn incomplete_input() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":3,\"val");

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn single_exact_input() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":3,\"value\":4}\n");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 3, value: 4 })
            ));
    }

    #[test]
    fn single_item_split_into_two_inputs() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":42,");
        engine.input("\"value\":24}\n");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 42, value: 24 })
            ));
    }

    #[test]
    fn two_items_in_single_input() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":1,\"value\":1}\n{\"key\":2,\"value\":2}\n");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 1, value: 1 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 2, value: 2 })
            ));
    }

    #[test]
    fn two_items_in_many_inputs_with_rest() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":12,\"v");
        engine.input("alue\":3");
        engine.input("4}\n{\"key");
        engine.input("\":56,\"valu");
        engine.input("e\":78}\n{\"key\":");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 12, value: 34 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 56, value: 78 })
            ));
    }

    #[test]
    fn input_completing_previous_rest_then_multiple_complete_items_and_more_rest() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":9,\"value\":");
        engine.input("8}\n{\"key\":7,\"value\":6}\n{\"key\":5,\"value\":4}\n{\"key\":");
        engine.input("3,\"value\":2}\n{");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 9, value: 8 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 7, value: 6 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 5, value: 4 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 3, value: 2 })
            ));
    }

    #[test]
    fn carriage_return_handled_gracefully() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":1,\"value\":2}\r\n{\"key\":3,\"value\":4}\r\n");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 1, value: 2 }),
                |it| assert_that!(it).contains_value(TestStruct { key: 3, value: 4 })
            ));
    }

    #[test]
    fn whitespace_handled_gracefully() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("\t{ \"key\":\t13,  \"value\":   37 } \r\n");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 13, value: 37 })
            ));
    }

    #[test]
    fn erroneous_entry_emitted_as_json_error() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":1}\n{\"key\":1,\"value\":1}\n");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).is_err(),
                |it| assert_that!(it).is_ok()
            ));
    }

    #[test]
    fn error_from_split_entry() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input("{\"key\":100,\"value\":200}\n{\"key\":");
        engine.input("\"should be a number\",\"value\":0}\n{\"key\":300,\"value\":400}\n");

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 100, value: 200 }),
                |it| assert_that!(it).is_err(),
                |it| assert_that!(it).contains_value(TestStruct { key: 300, value: 400 })
            ));
    }

    #[test]
    fn engine_input_works_for_different_types() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::default();

        engine.input(b"{\"k");
        engine.input(b"ey\"".to_vec());
        engine.input(":12".to_string());
        engine.input(&mut ",\"v".to_string());
        engine.input("alu".to_string().into_boxed_str());
        engine.input(b"e\"".to_vec().into_boxed_slice());
        engine.input(Arc::<str>::from(":3"));
        engine.input(Rc::<[u8]>::from(&b"4}"[..]));
        engine.input(Cow::Borrowed(&b"\r\n".to_vec()));

        assert_that!(collect_output(engine))
            .satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 12, value: 34 })
            ));
    }

    #[test]
    fn old_data_is_discarded() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();
        let count = 20;

        engine.input("{ \"key\": 1, ");

        for _ in 0..(count - 1) {
            engine.input("\"value\": 2 }\r\n{ \"key\": 1, ");
        }

        engine.input("\"value\": 2 }\r\n");

        assert_that!(engine.in_queue).is_empty();
        assert_that!(engine.out_queue).has_length(count);
    }

    fn configured_engine(configure: impl FnOnce(NdjsonConfig) -> NdjsonConfig)
            -> NdjsonEngine<TestStruct> {
        let config = configure(NdjsonConfig::default());
        NdjsonEngine::with_config(config)
    }

    fn engine_with_empty_line_handling(empty_line_handling: EmptyLineHandling)
            -> NdjsonEngine<TestStruct> {
        configured_engine(|config| config.with_empty_line_handling(empty_line_handling))
    }

    #[test]
    fn raises_error_when_parsing_empty_line_in_parse_always_mode() {
        let mut engine = engine_with_empty_line_handling(EmptyLineHandling::ParseAlways);

        engine.input("{\"key\":1,\"value\":2}\n\n{\"key\":3,\"value\":4}\n");

        assert_that!(collect_output(engine)).contains_elements_matching(Result::is_err);
    }

    #[test]
    fn does_not_raise_error_when_parsing_empty_line_in_ignore_empty_mode() {
        let mut engine = engine_with_empty_line_handling(EmptyLineHandling::IgnoreEmpty);

        engine.input("{\"key\":1,\"value\":2}\n\n{\"key\":3,\"value\":4}\n");

        assert_that!(collect_output(engine)).does_not_contain_elements_matching(Result::is_err);
    }

    #[test]
    fn does_not_raise_error_when_parsing_empty_line_with_carriage_return_in_ignore_empty_mode() {
        let mut engine = engine_with_empty_line_handling(EmptyLineHandling::IgnoreEmpty);

        engine.input("{\"key\":1,\"value\":2}\r\n\r\n{\"key\":3,\"value\":4}\n");

        assert_that!(collect_output(engine)).does_not_contain_elements_matching(Result::is_err);
    }

    #[test]
    fn raises_error_when_parsing_non_empty_blank_line_in_ignore_empty_mode() {
        let mut engine = engine_with_empty_line_handling(EmptyLineHandling::IgnoreEmpty);

        engine.input("{\"key\":1,\"value\":2}\n \t\r\n{\"key\":3,\"value\":4}\n");

        assert_that!(collect_output(engine)).contains_elements_matching(Result::is_err);
    }

    #[test]
    fn does_not_raise_error_when_parsing_non_empty_blank_line_in_ignore_blank_mode() {
        let mut engine = engine_with_empty_line_handling(EmptyLineHandling::IgnoreBlank);

        engine.input("{\"key\":1,\"value\":2}\n \t\r\n{\"key\":3,\"value\":4}\n");

        assert_that!(collect_output(engine)).does_not_contain_elements_matching(Result::is_err);
    }

    #[test]
    fn finalize_ignores_rest_if_parse_rest_is_false() {
        let mut engine = configured_engine(|config| config.with_parse_rest(false));

        engine.input("{\"key\":1,\"value\":2}");
        engine.finalize();

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn finalize_parses_valid_rest() {
        const EMPTY_LINE_HANDLINGS: [EmptyLineHandling; 3] = [
            EmptyLineHandling::ParseAlways,
            EmptyLineHandling::IgnoreEmpty,
            EmptyLineHandling::IgnoreBlank
        ];

        for empty_line_handling in EMPTY_LINE_HANDLINGS {
            let mut engine = configured_engine(|config| config
                .with_empty_line_handling(empty_line_handling)
                .with_parse_rest(true));

            engine.input("{\"key\":1,\"value\":2}");
            engine.finalize();

            assert_that!(collect_output(engine)).satisfies_exactly_in_given_order(dyn_assertions!(
                |it| assert_that!(it).contains_value(TestStruct { key: 1, value: 2 })
            ));
        }
    }

    #[test]
    fn finalize_raises_error_on_invalid_rest() {
        let mut engine = configured_engine(|config| config.with_parse_rest(true));

        engine.input("invalid json");
        engine.finalize();

        assert_that!(collect_output(engine)).satisfies_exactly_in_given_order(dyn_assertions!(
            |it| assert_that!(it).is_err()
        ));
    }

    #[test]
    fn finalize_ignores_empty_rest_even_if_empty_line_handling_is_parse_always() {
        let mut engine = configured_engine(|config| config
            .with_empty_line_handling(EmptyLineHandling::ParseAlways)
            .with_parse_rest(true));

        engine.finalize();

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn finalize_ignores_empty_rest_if_empty_line_handling_is_ignore_empty() {
        let mut engine = configured_engine(|config| config
            .with_empty_line_handling(EmptyLineHandling::IgnoreEmpty)
            .with_parse_rest(true));

        engine.finalize();

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn finalize_does_not_ignore_non_empty_blank_rest_if_empty_line_handling_is_ignore_empty() {
        let mut engine = configured_engine(|config| config
            .with_empty_line_handling(EmptyLineHandling::IgnoreEmpty)
            .with_parse_rest(true));

        engine.input(" ");
        engine.finalize();

        assert_that!(collect_output(engine)).satisfies_exactly_in_given_order(dyn_assertions!(
            |it| assert_that!(it).is_err()
        ));
    }

    #[test]
    fn finalize_ignores_non_empty_blank_rest_if_empty_line_handling_is_ignore_blank() {
        let mut engine = configured_engine(|config| config
            .with_empty_line_handling(EmptyLineHandling::IgnoreBlank)
            .with_parse_rest(true));

        engine.input(" ");
        engine.finalize();

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn finalize_is_idempotent() {
        let mut engine = configured_engine(|config| config.with_parse_rest(true));

        engine.input("{\"key\":13,\"value\":37}");
        engine.finalize();
        engine.finalize();

        assert_that!(collect_output(engine)).satisfies_exactly_in_given_order(dyn_assertions!(
            |it| assert_that!(it).contains_value(TestStruct { key: 13, value: 37 })
        ));
    }
}
