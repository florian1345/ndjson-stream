use std::collections::VecDeque;

use serde::Deserialize;

use serde_json::error::Result as JsonResult;
use crate::bytes::AsBytes;

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
    out_queue: VecDeque<JsonResult<T>>
}

impl<T> NdjsonEngine<T> {

    /// Creates a new NDJSON-engine for objects of the given type parameter.
    pub fn new() -> NdjsonEngine<T> {
        NdjsonEngine {
            in_queue: Vec::new(),
            out_queue: VecDeque::new()
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

            // TODO error handling, whitespace handling (?)
            let next_item = serde_json::from_slice(next_item_bytes);
            self.out_queue.push_back(next_item);

            self.in_queue.clear();
            data = &data[(newline_idx + 1)..];
        }

        self.in_queue.extend_from_slice(data);
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

        assert_that!(engine.in_queue.len()).is_zero();
        assert_that!(engine.out_queue.len()).is_equal_to(count);
    }
}
