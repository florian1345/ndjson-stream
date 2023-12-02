# ndjson-stream

`ndjson-stream` offers a variety of NDJSON-parsers which accept data in chunks and process these chunks before reading
further, thus enabling a streaming-style use.
The parser accepts a variety of inputs which represent byte slices, e.g. `Vec<u8>` or `&str`.
`ndjson-stream` uses the [serde_json](https://crates.io/crates/serde_json) crate to parse individual lines.

## High-level example

As an example, we will look at the iterator interface.
The most basic form can be instantiated with `from_iter`.
We have to provide an iterator over data blocks, and obtain an iterator over parsed NDJSON-records.
Actually, the exact return type is a `Result` which may contain a JSON-error in case a line is not valid JSON or does
not match the schema of the output type.

The example below demonstrates both the happy-path and parsing errors.

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Person {
    name: String,
    age: u16
}

let data_blocks = vec![
    "{\"name\":\"Alice\",\"age\":25}\n",
    "{\"this\":\"is\",\"not\":\"valid\"}\n",
    "{\"name\":\"Bob\",",
    "\"age\":35}\r\n"
];

let mut ndjson_iter = ndjson_stream::from_iter::<Person, _>(data_blocks);

assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Alice".into(), age: 25 });
assert!(ndjson_iter.next().unwrap().is_err());
assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Bob".into(), age: 35 });
assert!(ndjson_iter.next().is_none());
```

## Configuration

There are several configuration options available to control how the parser behaves in certain
situations.

In the example below, we construct an NDJSON-iterator which ignores blank lines.
That is, it does not produce an output record for any line which consists only of whitespace rather than attempting to
parse it and raising a JSON-error.

```rust
use ndjson_stream::config::{EmptyLineHandling, NdjsonConfig};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Person {
    name: String,
    age: u16
}

let data_blocks = vec![
    "{\"name\":\"Charlie\",\"age\":32}\n",
    "   \n",
    "{\"name\":\"Dolores\",\"age\":41}\n"
];
let config = NdjsonConfig::default().with_empty_line_handling(EmptyLineHandling::IgnoreBlank);

let mut ndjson_iter = ndjson_stream::from_iter_with_config::<Person, _>(data_blocks, config);

assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Charlie".into(), age: 32 });
assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Dolores".into(), age: 41 });
assert!(ndjson_iter.next().is_none());
```

## Fallibility

In addition to the ordinary interfaces, there is a fallible counterpart for each one.
"Fallible" in this context refers to the input data source - in the examples above the iterator of `data_blocks`.
Fallible parsers accept as input a data source which returns `Result`s with some error type and forward potential read
errors to the user.

In the example below, we use a fallible iterator.

```rust
use ndjson_stream::fallible::FallibleNdjsonError;
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Person {
    name: String,
    age: u16
}

let data_blocks = vec![
    Ok("{\"name\":\"Eve\",\"age\":22}\n"),
    Err("error"),
    Ok("{\"invalid\":json}\n")
];

let mut ndjson_iter = ndjson_stream::from_fallible_iter::<Person, _>(data_blocks);

assert_eq!(ndjson_iter.next().unwrap().unwrap(), Person { name: "Eve".into(), age: 22 });
assert!(matches!(ndjson_iter.next(), Some(Err(FallibleNdjsonError::InputError("error")))));
assert!(matches!(ndjson_iter.next(), Some(Err(FallibleNdjsonError::JsonError(_)))));
assert!(ndjson_iter.next().is_none());
```

For further information on how to use the `ndjson-stream` crate, view the [crate documentation][documentation].

## Links

* [Crate](https://crates.io/crates/ndjson-stream)
* [Documentation][documentation]
* [Repository](https://github.com/florian1345/ndjson-stream)

[documentation]: https://docs.rs/ndjson-stream/latest/ndjson-stream/
