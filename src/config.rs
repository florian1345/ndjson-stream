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
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NdjsonConfig {
    pub(crate) empty_line_handling: EmptyLineHandling
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
            empty_line_handling
        }
    }
}
