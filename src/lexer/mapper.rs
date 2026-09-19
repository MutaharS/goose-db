use crate::lexer::scanner::{RawKind, Scanner, Segment};

/// SQL reserved words recognized by the tokenizer.
///
/// Matching is case-insensitive, so `select`, `SELECT`, and `Select` all map
/// to the same variant.
#[derive(Debug)]
pub enum Keyword {
    Select,
    From,
    Insert,
    Into,
    Values,
    Where,
}

/// Structural punctuation and single-character operators.
#[derive(Debug)]
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
    /// `;`
    Semicolon,
}

/// A single classified token produced by the tokenizer.
///
/// Tokens borrow their payload text directly from the input string
/// (zero-copy); the `'a` lifetime ties the token to the source query it was
/// tokenized from.
#[derive(Debug)]
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

            // TODO: multi-char ops (<=, !=, <>) need a scanner rule, not a mapper arm

            // TODO: add operators and remaining punctuation.
            _ => SQLToken::Undefined(segment.text),
        },
    }
}

/// Tokenizes an entire input by pulling segments from `scanner` and mapping
/// each one to a [`SQLToken`].
///
/// Consumes the scanner; once the end of input is reached, further calls on
/// the same scanner return an empty vector.
pub fn tokenize<'a>(scanner: &mut Scanner<'a>) -> Vec<SQLToken<'a>> {
    let mut tokens = Vec::new();

    while let Some(segment) = scanner.next_segment() {
        tokens.push(token_mapper(&segment));
    }

    tokens
}
