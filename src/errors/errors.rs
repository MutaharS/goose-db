use std::error::Error;
use std::fmt;

/// The tokenizer could not map a scanned segment to any known token type.
///
/// Defined for future use; the tokenizer currently marks unknown input as
/// `SQLToken::Undefined` instead of returning this error.
#[derive(Debug)]
pub struct ScanError;

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ran into a scanner error. TODO: better error message")
    }
}

impl Error for ScanError {}
