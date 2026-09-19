use std::error::Error;
use std::fmt;

/// The tokenizer could not map a scanned segment to any known token type.
///
/// Defined for future use; the tokenizer currently marks unknown input as
/// `SQLToken::Undefined` instead of returning this error.
#[derive(Debug)]
pub struct MappingError;

impl fmt::Display for MappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ran into a mapping error during tokenization. Unable to map an input to SQLToken"
        )
    }
}

impl Error for MappingError {}
