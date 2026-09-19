/// How a segment was scanned, decided purely by its character structure.
///
/// This is a low-level hint only: it records *how* a run of characters was
/// delimited, not what the text means. Semantic classification (keyword vs
/// identifier vs literal) is the mapper's job.
#[derive(Debug, PartialEq)]
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
#[derive(Debug)]
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
    pub fn next_segment(&mut self) -> Option<Segment<'a>> {
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
        todo!("scan a raw run")
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
}
