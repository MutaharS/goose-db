use crate::errors::errors::ScanError;

/// How a segment was scanned, decided purely by its character structure.
///
/// This is a low-level hint only: it records *how* a run of characters was
/// delimited, not what the text means. Semantic classification (keyword vs
/// identifier vs literal) is the mapper's job.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RawKind {
    /// A run of letters, digits, and underscores, e.g. `user_id`.
    Word,
    /// A run of digits, optionally with a decimal point, e.g. `42`.
    Number,
    /// A run captured between quoting characters, e.g. `'active'`.
    Quoted,
    /// A single punctuation or operator character, e.g. `,` or `=`.
    Symbol,
}

/// A raw run of input produced by a [`Scanner`].
///
/// `text` is borrowed directly from the source, so scanning allocates
/// nothing. The position fields exist so that future parser error messages
/// can point at an exact location in the input.
#[derive(Debug, PartialEq)]
pub struct Segment<'a> {
    /// The raw text of the run, borrowed from the input.
    pub text: &'a str,
    /// Byte offset of the run within the input string.
    pub start: usize,
    /// 1-based line number where the run begins.
    pub line: u32,
    /// 1-based column number where the run begins.
    pub col: u32,
    /// Low-level hint describing how the run was delimited.
    pub kind: RawKind,
}

/// A stateful, char-by-char iterator over an input string that emits
/// [`Segment`]s.
///
/// The scanner tracks the current byte offset and the line/column position.
/// It owns the *structural* rules of tokenization (whitespace, quoting, run
/// boundaries) and knows nothing about SQL meaning.
#[derive(Debug)]
pub struct Scanner<'a> {
    input: &'a str,
    pos: usize,
    line: u32,
    col: u32,
}

// " SELECT   user_id  FROM table WHERE col = 'rejuvenating drink' AND year = '2026'  "
impl<'a> Scanner<'a> {
    /// Creates a scanner positioned at the very start of `input`.
    pub fn new(input: &'a str) -> Self {
        Scanner {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Scans and returns the next meaningful segment, or `None` at end of input.
    pub fn next_segment(&mut self) -> Result<Option<Segment<'a>>, ScanError> {
        // TODO:
        //   1. Skip any leading whitespace (keeping `line`/`col` correct while doing so).
        //   2. Decide the RawKind from the first remaining char:
        //        letter or '_'  -> Word
        //        digit          -> Number
        //        quote          -> Quoted
        //        anything else  -> Symbol
        //   3. Consume chars for as long as the run *continues* under that kind's
        //      boundary rules (for Quoted, that means until the closing quote).
        //   4. Emit Segment { text: &self.input[start..self.pos], start, line, col, kind }.

        // Consume whitespace
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }

        // Build segment
        let start = self.pos; // Set the start at the current position
        let start_line = self.line;
        let start_col = self.col;

        // Build next Segment, or possibly we gracefully exit if we have reached the end of the input stream
        let Some(first) = self.peek() else {
            return Ok(None);
        };

        // Decide the kind
        let kind: RawKind;
        if first.is_alphabetic() || first == '_' {
            kind = RawKind::Word;
        } else if first.is_ascii_digit() {
            kind = RawKind::Number;
        } else if first == '\'' {
            kind = RawKind::Quoted;
        } else {
            kind = RawKind::Symbol
        }

        // We consume characters for this segment based on the RawKind
        self.consume_segment(kind, start_line, start_col)?;

        // Build the segment to return
        let segment = Segment {
            text: self
                .input
                .get(start..self.pos)
                .expect("scanner positions always stay on valid UTF-8 boundaries"),
            start: start,
            line: start_line,
            col: start_col,
            kind: kind,
        };

        return Ok(Some(segment));
    }

    /// Peeks the next character without consuming it.
    fn peek(&self) -> Option<char> {
        self.input.get(self.pos..)?.chars().next()
    }

    /// Consumes one character, tracking byte position, line, and column.
    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn consume_segment(
        &mut self,
        kind: RawKind,
        start_line: u32,
        start_col: u32,
    ) -> Result<(), ScanError> {
        // Always consume the first (kind-deciding) character
        let first = self.advance();

        match kind {
            RawKind::Quoted => loop {
                match self.peek() {
                    Some('\'') => {
                        self.advance(); // consume the closing quote
                        break;
                    }
                    Some(_) => {
                        self.advance(); // part of the literal body
                    }
                    None => {
                        return Err(ScanError::UnterminatedStringLiteral {
                            line: start_line,
                            col: start_col,
                        });
                    } // unterminated string literal
                }
            },

            RawKind::Word => loop {
                // TODO: consume while the run continues (alphanumeric / '_')
                match self.peek() {
                    Some(val) if val == '_' || val.is_alphanumeric() => {
                        self.advance();
                    }
                    _ => break,
                }
            },

            RawKind::Number => {
                // Consume the digit run.
                while let Some(val) = self.peek() {
                    if val.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }

                // An identifier cannot start with a digit, so a letter or
                // underscore immediately after the digits is a malformed
                // token (e.g. `123abc`).
                if matches!(self.peek(), Some(c) if c.is_alphanumeric() || c == '_') {
                    return Err(ScanError::IdentifierStartsWithDigit {
                        line: start_line,
                        col: start_col,
                    });
                }
            }

            RawKind::Symbol => {
                // Maximal munch: `<` `>` `!` may take a second char to form a
                // two-char operator. `first` is non-empty whenever a kind was decided
                let first = first.expect("symbol kind implies a first char");
                if let Some(second) = self.peek()
                    && matches!(
                        (first, second),
                        ('<', '=') | ('<', '>') | ('>', '=') | ('!', '=')
                    )
                {
                    self.advance();
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_returns_none_at_end_of_input() {
        let scanner = Scanner::new("");
        assert_eq!(scanner.peek(), None);
    }

    #[test]
    fn peek_does_not_consume() {
        let scanner = Scanner::new("ab");
        assert_eq!(scanner.peek(), Some('a'));
        assert_eq!(scanner.peek(), Some('a'));
        assert_eq!(scanner.pos, 0);
    }

    #[test]
    fn advance_consumes_chars_in_order() {
        let mut scanner = Scanner::new("ab");
        assert_eq!(scanner.advance(), Some('a'));
        assert_eq!(scanner.advance(), Some('b'));
        assert_eq!(scanner.advance(), None);
    }

    #[test]
    fn advance_tracks_line_and_col_across_newlines() {
        let mut scanner = Scanner::new("a\nbb");
        scanner.advance();
        assert_eq!((scanner.line, scanner.col), (1, 2));

        scanner.advance();
        assert_eq!((scanner.line, scanner.col), (2, 1));

        scanner.advance();
        assert_eq!((scanner.line, scanner.col), (2, 2));
    }

    #[test]
    fn advance_counts_multibyte_utf8_as_one_char() {
        let mut scanner = Scanner::new("é");
        assert_eq!(scanner.advance(), Some('é'));
        assert_eq!(scanner.pos, 2);
    }

    #[test]
    fn next_segment_is_none_for_empty_input() {
        let mut scanner = Scanner::new("");
        assert_eq!(scanner.next_segment().unwrap(), None);
    }

    #[test]
    fn next_segment_is_none_for_whitespace_only_input() {
        let mut scanner = Scanner::new("   \n\t ");
        assert_eq!(scanner.next_segment().unwrap(), None);
    }

    #[test]
    fn scans_a_word_with_position() {
        let mut scanner = Scanner::new("page");
        let segment = scanner.next_segment().unwrap().unwrap();
        assert_eq!(
            segment,
            Segment {
                text: "page",
                start: 0,
                line: 1,
                col: 1,
                kind: RawKind::Word,
            }
        );
    }

    #[test]
    fn word_segment_records_start_position_after_whitespace() {
        let mut scanner = Scanner::new("  co");
        let segment = scanner.next_segment().unwrap().unwrap();
        assert_eq!(segment.text, "co");
        assert_eq!(segment.start, 2);
        assert_eq!(segment.col, 3);
    }

    #[test]
    fn scans_a_quoted_string() {
        let mut scanner = Scanner::new("'active'");
        let segment = scanner.next_segment().unwrap().unwrap();
        assert_eq!(
            segment,
            Segment {
                text: "'active'",
                start: 0,
                line: 1,
                col: 1,
                kind: RawKind::Quoted,
            }
        );
    }

    #[test]
    fn quoted_string_with_spaces_is_one_segment() {
        let mut scanner = Scanner::new("'rejuvenating drink'");
        let segment = scanner.next_segment().unwrap().unwrap();
        assert_eq!(segment.kind, RawKind::Quoted);
        assert_eq!(segment.text, "'rejuvenating drink'");
    }

    #[test]
    fn unterminated_quoted_string_is_an_error() {
        let mut scanner = Scanner::new("'oops");
        assert!(scanner.next_segment().is_err());
    }

    #[test]
    fn scans_a_number() {
        let mut scanner = Scanner::new("42");
        let segment = scanner.next_segment().unwrap().unwrap();
        assert_eq!(
            segment,
            Segment {
                text: "42",
                start: 0,
                line: 1,
                col: 1,
                kind: RawKind::Number,
            }
        );
    }

    #[test]
    fn scans_a_single_char_symbol() {
        let mut scanner = Scanner::new(",");
        let segment = scanner.next_segment().unwrap().unwrap();
        assert_eq!(
            segment,
            Segment {
                text: ",",
                start: 0,
                line: 1,
                col: 1,
                kind: RawKind::Symbol,
            }
        );
    }

    #[test]
    fn word_boundary_stops_at_symbol() {
        let mut scanner = Scanner::new("a,");
        let word = scanner.next_segment().unwrap().unwrap();
        assert_eq!(word.text, "a");
        assert_eq!(word.kind, RawKind::Word);

        let symbol = scanner.next_segment().unwrap().unwrap();
        assert_eq!(symbol.text, ",");
        assert_eq!(symbol.kind, RawKind::Symbol);
    }

    #[test]
    fn digit_run_followed_by_letter_is_an_error() {
        let mut scanner = Scanner::new("123abc");
        assert!(scanner.next_segment().is_err());
    }

    #[test]
    fn digit_run_followed_by_underscore_is_an_error() {
        let mut scanner = Scanner::new("42_elephant");
        assert!(scanner.next_segment().is_err());
    }

    #[test]
    fn digit_run_before_symbol_is_not_an_error() {
        let mut scanner = Scanner::new("123,");
        let number = scanner.next_segment().unwrap().unwrap();
        assert_eq!(number.text, "123");
        assert_eq!(number.kind, RawKind::Number);

        let symbol = scanner.next_segment().unwrap().unwrap();
        assert_eq!(symbol.text, ",");
        assert_eq!(symbol.kind, RawKind::Symbol);
    }

    fn segments_of(input: &str) -> Vec<(RawKind, &str)> {
        let mut scanner = Scanner::new(input);
        let mut out = Vec::new();
        while let Some(segment) = scanner.next_segment().unwrap() {
            out.push((segment.kind, segment.text));
        }
        out
    }

    #[test]
    fn two_char_operators_scan_as_single_symbols() {
        let tokens = segments_of("a<=b, c>=d, e<>f, g!=h");
        assert_eq!(
            tokens,
            vec![
                (RawKind::Word, "a"),
                (RawKind::Symbol, "<="),
                (RawKind::Word, "b"),
                (RawKind::Symbol, ","),
                (RawKind::Word, "c"),
                (RawKind::Symbol, ">="),
                (RawKind::Word, "d"),
                (RawKind::Symbol, ","),
                (RawKind::Word, "e"),
                (RawKind::Symbol, "<>"),
                (RawKind::Word, "f"),
                (RawKind::Symbol, ","),
                (RawKind::Word, "g"),
                (RawKind::Symbol, "!="),
                (RawKind::Word, "h"),
            ]
        );
    }

    #[test]
    fn non_operator_symbol_pairs_do_not_fuse() {
        let tokens = segments_of("a == b => c !> d");
        assert_eq!(
            tokens,
            vec![
                (RawKind::Word, "a"),
                (RawKind::Symbol, "="),
                (RawKind::Symbol, "="),
                (RawKind::Word, "b"),
                (RawKind::Symbol, "="),
                (RawKind::Symbol, ">"),
                (RawKind::Word, "c"),
                (RawKind::Symbol, "!"),
                (RawKind::Symbol, ">"),
                (RawKind::Word, "d"),
            ]
        );
    }

    #[test]
    fn bang_alone_stays_a_single_symbol() {
        let tokens = segments_of("a!");
        assert_eq!(tokens, vec![(RawKind::Word, "a"), (RawKind::Symbol, "!")]);
    }

    #[test]
    fn operator_fusion_is_independent_of_whitespace() {
        assert_eq!(segments_of("a<=b"), segments_of("a <= b"));
        assert_eq!(segments_of("c!=d"), segments_of("c != d"));
    }
}
