use std::error::Error;
use std::fmt;

/// The tokenizer could not map a scanned segment to any known token type.
///
/// Defined for future use; the tokenizer currently marks unknown input as
/// `SQLToken::Undefined` instead of returning this error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScanError {
    /// Reached end of input before the closing quote.
    UnterminatedStringLiteral { line: u32, col: u32 },
    /// A token started with digits and ran into identifier characters, e.g. `123abc`.
    IdentifierStartsWithDigit { line: u32, col: u32 },
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanError::UnterminatedStringLiteral { line, col } => {
                write!(
                    f,
                    "unterminated string literal at line {line}, column {col}"
                )
            }
            ScanError::IdentifierStartsWithDigit { line, col } => {
                write!(
                    f,
                    "identifier cannot start with a digit at line {line}, column {col}"
                )
            }
        }
    }
}

impl Error for ScanError {}
