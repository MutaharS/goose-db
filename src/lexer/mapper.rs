use crate::errors::errors::ScanError;
use crate::lexer::scanner::{RawKind, Scanner, Segment};

/// SQL reserved words recognized by the tokenizer.
///
/// Matching is case-insensitive, so `select`, `SELECT`, and `Select` all map
/// to the same variant.
#[derive(Debug, PartialEq)]
pub enum Keyword {
    Select,
    From,
    Insert,
    Into,
    Values,
    Where,
}

/// Structural punctuation, single- and multi-character operators.
#[derive(Debug, PartialEq)]
pub enum Punctuation {
    /// `*`
    Star,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `(`
    Lparen,
    /// `)`
    Rparen,
    /// `=`
    Equals,
    /// `<`
    LessThan,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanEqual,
    /// `<>`
    NotEqual,
    /// `<=`
    LessThanEqual,
    /// `;`
    Semicolon,
}

/// A single classified token produced by the tokenizer.
///
/// Tokens borrow their payload text directly from the input string
/// (zero-copy); the `'a` lifetime ties the token to the source query it was
/// tokenized from.
#[derive(Debug, PartialEq)]
pub enum SQLToken<'a> {
    /// A reserved word, e.g. `SELECT` or `WHERE`.
    Keyword(Keyword),
    /// Structural syntax, e.g. `,`, `(`, or `=`.
    Punctuation(Punctuation),
    /// A table or column name, e.g. `user_id`.
    Identifier(&'a str),
    /// A single-quoted string value, e.g. `'active'`.
    StringLiteral(&'a str),
    /// A numeric value, e.g. `42` or `3.14`.
    NumericLiteral(&'a str),
    /// A segment that was scanned but has no classification yet.
    Undefined(&'a str),
}

/// Classifies a raw [`Segment`] into a high-level [`SQLToken`].
///
/// The mapper is a pure function: the same segment always maps to the same
/// token. Keyword detection is case-insensitive via
/// [`str::eq_ignore_ascii_case`]; anything unrecognized stays `Undefined`
/// rather than failing.
fn token_mapper<'a>(segment: &Segment<'a>) -> SQLToken<'a> {
    match segment.kind {
        RawKind::Word => {
            if segment.text.eq_ignore_ascii_case("SELECT") {
                SQLToken::Keyword(Keyword::Select)
            } else if segment.text.eq_ignore_ascii_case("FROM") {
                SQLToken::Keyword(Keyword::From)
            } else if segment.text.eq_ignore_ascii_case("INSERT") {
                SQLToken::Keyword(Keyword::Insert)
            } else if segment.text.eq_ignore_ascii_case("INTO") {
                SQLToken::Keyword(Keyword::Into)
            } else if segment.text.eq_ignore_ascii_case("VALUES") {
                SQLToken::Keyword(Keyword::Values)
            } else if segment.text.eq_ignore_ascii_case("WHERE") {
                SQLToken::Keyword(Keyword::Where)
            } else {
                SQLToken::Identifier(segment.text)
            }
        }

        RawKind::Quoted => {
            // TODO: decide whether to strip the surrounding quotes here.
            SQLToken::StringLiteral(segment.text)
        }

        RawKind::Number => {
            // TODO: classify numeric literals (int vs float, or just Literal for now).
            SQLToken::NumericLiteral(segment.text)
        }

        RawKind::Symbol => match segment.text {
            "*" => SQLToken::Punctuation(Punctuation::Star),
            "," => SQLToken::Punctuation(Punctuation::Comma),
            "." => SQLToken::Punctuation(Punctuation::Dot),
            "(" => SQLToken::Punctuation(Punctuation::Lparen),
            ")" => SQLToken::Punctuation(Punctuation::Rparen),
            "=" => SQLToken::Punctuation(Punctuation::Equals),
            ";" => SQLToken::Punctuation(Punctuation::Semicolon),
            "<" => SQLToken::Punctuation(Punctuation::LessThan),
            ">" => SQLToken::Punctuation(Punctuation::GreaterThan),

            // multi-char ops (<=, !=, <>)
            "<>" => SQLToken::Punctuation(Punctuation::NotEqual),
            "!=" => SQLToken::Punctuation(Punctuation::NotEqual),
            ">=" => SQLToken::Punctuation(Punctuation::GreaterThanEqual),
            "<=" => SQLToken::Punctuation(Punctuation::LessThanEqual),

            // TODO: add operators and remaining punctuation.
            _ => SQLToken::Undefined(segment.text),
        },
    }
}

/// Tokenizes an entire input by pulling segments from `scanner` and mapping
/// each one to a [`SQLToken`].
///
/// Stops cleanly at end of input; tokenizing an already-drained scanner
/// yields an empty vector. Any scanning error is propagated to the caller
/// rather than swallowed here.
pub fn tokenize<'a>(scanner: &mut Scanner<'a>) -> Result<Vec<SQLToken<'a>>, ScanError> {
    let mut tokens = Vec::new();

    while let Some(segment) = scanner.next_segment()? {
        tokens.push(token_mapper(&segment));
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(text: &'static str, kind: RawKind) -> SQLToken<'static> {
        token_mapper(&Segment {
            text,
            start: 0,
            line: 1,
            col: 1,
            kind,
        })
    }

    #[test]
    fn select_keyword_is_case_insensitive() {
        assert_eq!(
            classify("SELECT", RawKind::Word),
            SQLToken::Keyword(Keyword::Select)
        );
        assert_eq!(
            classify("select", RawKind::Word),
            SQLToken::Keyword(Keyword::Select)
        );
        assert_eq!(
            classify("SeLeCt", RawKind::Word),
            SQLToken::Keyword(Keyword::Select)
        );
    }

    #[test]
    fn where_keyword_is_recognized() {
        assert_eq!(
            classify("WHERE", RawKind::Word),
            SQLToken::Keyword(Keyword::Where)
        );
    }

    #[test]
    fn unknown_word_is_an_identifier() {
        assert_eq!(
            classify("user_id", RawKind::Word),
            SQLToken::Identifier("user_id")
        );
    }

    #[test]
    fn quoted_segment_is_a_string_literal() {
        assert_eq!(
            classify("'active'", RawKind::Quoted),
            SQLToken::StringLiteral("'active'")
        );
    }

    #[test]
    fn numeric_segment_is_a_numeric_literal() {
        assert_eq!(
            classify("42", RawKind::Number),
            SQLToken::NumericLiteral("42")
        );
    }

    #[test]
    fn known_symbols_map_to_punctuation() {
        assert_eq!(
            classify(",", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::Comma)
        );
        assert_eq!(
            classify("=", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::Equals)
        );
        assert_eq!(
            classify("*", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::Star)
        );
        assert_eq!(
            classify(";", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::Semicolon)
        );
    }

    #[test]
    fn multi_char_operators_map_to_punctuation() {
        assert_eq!(
            classify("<=", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::LessThanEqual)
        );
        assert_eq!(
            classify(">=", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::GreaterThanEqual)
        );
        assert_eq!(
            classify("<>", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::NotEqual)
        );
        assert_eq!(
            classify("!=", RawKind::Symbol),
            SQLToken::Punctuation(Punctuation::NotEqual)
        );
    }

    #[test]
    fn unknown_symbol_stays_undefined() {
        assert_eq!(classify("@", RawKind::Symbol), SQLToken::Undefined("@"));
    }
}
