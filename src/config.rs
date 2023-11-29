/// Controls how the parser deals with lines that contain no JSON values.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum EmptyLineHandling {

    /// Parse every line, i.e. every segment between `\n` characters, even if it is empty. This will
    /// result in errors for empty lines.
    #[default]
    ParseAlways,

    /// Ignore lines, i.e. segments between `\n` characters, which are empty, i.e. contain no
    /// characters. For compatibility with `\r\n`-style linebreaks, this also ignores lines which
    /// consist of only a single `\r` character.
    IgnoreEmpty,

    /// Ignore lines, i.e. segments between `\n` characters, which contain only whitespace
    /// characters.
    IgnoreBlank
}

/// Configuration for the NDJSON-parser which controls the behavior in various situations.
///
/// By default, the parser will attempt to parse every line, i.e. every segment between `\n`
/// characters, even if it is empty. This will result in errors for empty lines.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NdjsonConfig {
    pub(crate) empty_line_handling: EmptyLineHandling,
    pub(crate) parse_rest: bool
}

impl NdjsonConfig {

    /// Creates a new config from this config which has a different handling for lines that contain
    /// no JSON values. See [EmptyLineHandling] for more details.
    ///
    /// # Returns
    ///
    /// A new config with all the same values as this one, except the empty-line-handling.
    pub fn with_empty_line_handling(self, empty_line_handling: EmptyLineHandling) -> NdjsonConfig {
        NdjsonConfig {
            empty_line_handling,
            ..self
        }
    }

    /// Creates a new config from this config which has the given configuration on whether to parse
    /// or ignore the rest, i.e. the part after the last newline character. If `parse_rest` is set
    /// to `false`, the rest will always be ignored, while `true` causes it to only be ignored if it
    /// is empty or considered empty by the handling configured in
    /// [NdjsonConfig::with_empty_line_handling], which by default is only truly empty. Otherwise,
    /// the rest is parsed like an ordinary JSON record. By default, this is set to `false`.
    ///
    /// # Returns
    ///
    /// A new config with all the same values as this one, except the parse-rest-flag.
    pub fn with_parse_rest(self, parse_rest: bool) -> NdjsonConfig {
        NdjsonConfig {
            parse_rest,
            ..self
        }
    }
}
