use std::collections::VecDeque;

use serde::Deserialize;

/// The low-level engine parsing NDJSON-data given as byte slices into objects of the type parameter
/// `T`. Data is supplied in chunks and parsed objects can subsequently be read from a queue.
///
/// Users of this crate should usually not have to use this struct but rather a higher-level
/// interface such as iterators.
pub struct NdjsonEngine<T> {
    in_queue: Vec<u8>,
    out_queue: VecDeque<T>
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
    /// supplied previously via [NdjsonEngine::input]. Otherwise, `None` is returned.
    pub fn next(&mut self) -> Option<T> {
        self.out_queue.pop_front()
    }
}

fn index_of<T: Eq>(data: &[T], search: T) -> Option<usize> {
    data.iter().enumerate()
        .find(|&(_, item)| item == &search)
        .map(|(index, _)| index)
}

const NEW_LINE: u8 = b'\n';

impl<T> NdjsonEngine<T>
where
    for<'deserialize> T: Deserialize<'deserialize>
{

    /// Parses the given data as NDJSON. In case the end does not match up with a newline, the rest
    /// is stored in an internal cache. Consequently, the rest from a previous call to this method
    /// is prepended to the given data in case a newline is encountered.
    pub fn input(&mut self, mut data: &[u8]) {
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
            let next_item = serde_json::from_slice(next_item_bytes).unwrap();
            self.out_queue.push_back(next_item);

            self.in_queue.clear();
            data = &data[(newline_idx + 1)..];
        }

        self.in_queue.extend_from_slice(data);
    }
}

#[cfg(test)]
mod tests {

    use crate::engine::NdjsonEngine;

    use kernal::prelude::*;

    use serde::Deserialize;

    use std::iter;

    #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
    struct TestStruct {
        key: u64,
        value: u64
    }

    fn collect_output(mut engine: NdjsonEngine<TestStruct>) -> Vec<TestStruct> {
        iter::from_fn(|| engine.next()).collect::<Vec<_>>()
    }

    #[test]
    fn no_input() {
        let engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn incomplete_input() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input(b"{\"key\":3,\"val");

        assert_that!(collect_output(engine)).is_empty();
    }

    #[test]
    fn single_exact_input() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input(b"{\"key\":3,\"value\":4}\n");

        assert_that!(collect_output(engine))
            .contains_exactly_in_given_order([TestStruct { key: 3, value: 4 }]);
    }

    #[test]
    fn single_item_split_into_two_inputs() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input(b"{\"key\":42,");
        engine.input(b"\"value\":24}\n");

        assert_that!(collect_output(engine))
            .contains_exactly_in_given_order([TestStruct { key: 42, value: 24 }]);
    }

    #[test]
    fn two_items_in_single_input() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input(b"{\"key\":1,\"value\":1}\n{\"key\":2,\"value\":2}\n");

        assert_that!(collect_output(engine))
            .contains_exactly_in_given_order([
                TestStruct { key: 1, value: 1 },
                TestStruct { key: 2, value: 2 }
            ]);
    }

    #[test]
    fn two_items_in_many_inputs_with_rest() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input(b"{\"key\":12,\"v");
        engine.input(b"alue\":3");
        engine.input(b"4}\n{\"key");
        engine.input(b"\":56,\"valu");
        engine.input(b"e\":78}\n{\"key\":");

        assert_that!(collect_output(engine))
            .contains_exactly_in_given_order([
                TestStruct { key: 12, value: 34 },
                TestStruct { key: 56, value: 78 }
            ]);
    }

    #[test]
    fn carriage_return_handled_gracefully() {
        let mut engine: NdjsonEngine<TestStruct> = NdjsonEngine::new();

        engine.input(b"{\"key\":1,\"value\":2}\r\n{\"key\":3,\"value\":4}\r\n");

        assert_that!(collect_output(engine))
            .contains_exactly_in_given_order([
                TestStruct { key: 1, value: 2 },
                TestStruct { key: 3, value: 4 }
            ]);
    }
}
