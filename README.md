# SQL Tokenizer Design

## Sample Query

`e.g. SELECT user_id, temperature FROM table WHERE status = 'active'`

## Overview

Each piece of a SQL query can be broken down into multiple tokens. Those tokens chained together
in a particular sequence can then form an expression. It may be that a single token on its own can also form an expression.
This design of a self-contained expression will assist as we design and make our features more complex - allowing for fluidity in the
way the language can be described and consequently parsed; this is just my hypothesis and will remain to be seen whether it holds true.

## Architecture: Two-Tier Lexer

Tokenization is split into two cooperating stages, mirroring how real lexers work:

```
input &str ──► Scanner ──► Segment ──► Mapper ──► SQLToken
              (splits)    (raw run)   (classifies)
```

1. **Scanner** (`src/lexer/scanner.rs`) — walks the input char-by-char and chops it into
   **segments**: raw runs of text, decided by *structural* rules only. It has no idea what
   `SELECT` means. It is also responsible for position tracking (byte offset, line, column)
   so future error messages can point at the right place.
2. **Mapper** (`src/lexer/mapper.rs`) — a pure function from `Segment` to `SQLToken`.
   It applies *semantic* rules: is this scanned word a known `Keyword`, or an `Identifier`
   instead? Same input, same output — no state.

The glue loop lives in `tokenize(scanner)` which keeps pulling segments and mapping them.

### Segments (`lexer::scanner`)

```rust
pub enum RawKind { Word, Number, Quoted, Symbol }

pub struct Segment<'a> {
    pub text: &'a str,   // raw text, borrowed straight from the input (zero-copy)
    pub start: usize,    // byte offset into the input
    pub line: u32,
    pub col: u32,        // for future error reporting
    pub kind: RawKind,   // low-level scan hint, NOT the final meaning
}
```

The scanner's vocabulary is intentionally *low level*:

| `RawKind` | How it was scanned |
|---|---|
| `Word`   | run of letters/underscore/digits, e.g. `user_id` |
| `Number` | run of digits (optionally with decimal point) |
| `Quoted` | everything captured inside quotes, e.g. `'active'` |
| `Symbol` | a single punctuation/operator char, e.g. `,` `=` `(` |

`Keyword`, `Identifier`, and `Literal` are **mapper** concepts and never appear here.

### Tokens (`lexer::mapper`)

```rust
pub enum SQLToken<'a> {
    Keyword(Keyword),        // Select, From, Insert, Into, Values, Where
    Punctuation(Punctuation),// Star, Comma, Dot, Lparen, Rparen, Equals
    Identifier(&'a str),     // user_id, table
    Literal(&'a str),        // 'active'
    Undefined(&'a str),      // scanned but not yet classified
}
```

The distinct `'a` lifetime on tokens keeps them borrowing from the original input —
tokenizing never allocates or copies strings.

## SQL CFG/PDA

To start, it makes sense to define the possible structures a SQL query can look like and breakdown the possible
expressions/tokens that comprise it. In other words, we need to create a Context Free Grammar / Push Down Automata that define a valid sequences of SQL tokens.

```ebnf
/* Top-Level Statements */
<Statement>       ::= <SelectExpr> | <InsertStmt>

/* Core Query Expressions */
<SelectExpr>      ::= "SELECT" <ColumnExpr> "FROM" <TableSource>
<TableSource>     ::= <TableName> | "(" <SelectExpr> ")"

<InsertStmt>      ::= "INSERT INTO" <TableName> <InsertBody>
<InsertBody>      ::= <ValuesExpr> | <SelectExpr>

/* Column Expressions */
<ColumnExpr>      ::= "*" | <ColumnList>
<ColumnList>      ::= <ColumnItem> | <ColumnItem> "," <ColumnList>
<ColumnItem>      ::= <Identifier> | <Identifier> "." <Identifier>

/* Table & Values Terms */
<TableName>       ::= <Identifier> | <Identifier> "." <Identifier>
<ValuesExpr>      ::= "VALUES" "(" <ValueList> ")"
<ValueList>       ::= <Literal> | <Literal> "," <ValueList>

/* Terminal Tokens (Handled by Tokenizer/Lexer) */
<Identifier>      ::= [a-zA-Z_][a-zA-Z0-9_]*
<Literal>         ::= STRING_LITERAL | NUMBER_LITERAL
```

## Current Status

Implemented and working:

- [x] `Scanner` struct with byte-offset / line / column tracking scaffolding
- [x] `Segment` + `RawKind` data model
- [x] `token_mapper` classified over `RawKind` (keywords are case-insensitive via `eq_ignore_ascii_case`)
- [x] Word fallback in the mapper: unknown word → `Identifier` instead of `Undefined`
- [x] `tokenize(scanner)` drive loop replacing the naive `split(' ')`

Under construction (open TODOs to expand):

- [ ] `Scanner::advance()` / `peek()` — char consumption with UTF-8 length + newline tracking
- [ ] `Scanner::next_segment()` — whitespace skipping, raw-run boundary rules, quoted-span handling
- [ ] `Number` classification, operator symbols, more `Punctuation` variants
- [ ] Decide: is `123abc` one segment or two (`Number` then `Word`)?
- [ ] Decide: do Whitespace/Comments get skipped by the scanner or emitted as segments?

## Module Layout

```
src/
├── main.rs              # wires Scanner → tokenize → print
├── errors/
│   └── errors.rs        # MappingError (not yet wired in)
└── lexer/
    ├── mod.rs
    ├── mapper.rs        # token_mapper + tokenize drive loop + SQLToken
    └── scanner.rs       # Scanner, Segment, RawKind
```