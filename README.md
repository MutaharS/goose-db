# goose-db

A hand-written SQL database built to learn Rust. Phase 1 — the tokenizer — is
complete and tested. Next phases add the parser (CFG/PDA) and execution.

## Sample Query

```sql
SELECT user_id, temperature FROM table WHERE status = 'active'
```

## Overview

Each piece of a SQL query can be broken down into multiple tokens. Those tokens
chained together in a particular sequence can then form an expression. It may be
that a single token on its own can also form an expression. This design of a
self-contained expression will assist as we design and make our features more
complex — allowing for fluidity in the way the language can be described and
consequently parsed; this is just my hypothesis and will remain to be seen
whether it holds true.

## Architecture: Two-Tier Lexer

Tokenization is split into two cooperating stages, mirroring how real lexers
work:

```
input &str ──► Scanner ──► Segment ──► Mapper ──► SQLToken
              (splits)    (raw run)   (classifies)
```

1. **Scanner** (`src/lexer/scanner.rs`) — walks the input char-by-char and chops
   it into **segments**: raw runs of text, decided by _structural_ rules only.
   It has no idea what `SELECT` means. It also tracks position (byte offset,
   line, column) so future error messages can point at the exact spot.
   Whitespace is skipped here, never emitted.
2. **Mapper** (`src/lexer/mapper.rs`) — a pure function from `Segment` to
   `SQLToken`. It applies _semantic_ rules: is this scanned word a known
   `Keyword`, or an `Identifier` instead? Same input, same output — no state.
3. **Error layer** (`src/errors/errors.rs`) — `ScanError` sits at the top of the
   stack; `tokenize` propagates scan failures instead of swallowing them.

The glue loop lives in `tokenize(scanner)`, which keeps pulling segments and
mapping them until end of input.

### Segments (`lexer::scanner`)

```rust
pub enum RawKind { Word, Number, Quoted, Symbol }   // Copy + PartialEq

pub struct Segment<'a> {                            // Debug + PartialEq
    pub text: &'a str,   // raw text, borrowed straight from the input (zero-copy)
    pub start: usize,    // byte offset where the run begins
    pub line: u32,       // 1-based, where the run begins
    pub col: u32,        // 1-based, where the run begins
    pub kind: RawKind,   // low-level scan hint, NOT the final meaning
}
```

The scanner's vocabulary is intentionally _low level_:

| `RawKind` | How it was scanned                                                                     |
| --------- | -------------------------------------------------------------------------------------- |
| `Word`    | run of letters/digits/underscore, e.g. `user_id`                                       |
| `Number`  | run of digits; a letter/underscore right after is an error (`123abc`)                  |
| `Quoted`  | everything up to the matching close quote, e.g. `'active'`; unterminated strings `Err` |
| `Symbol`  | a single punctuation/operator char, e.g. `,` `=` `(` `*`                               |

`Keyword`, `Identifier`, and `Literal` are **mapper** concepts and never appear here.

### Tokens (`lexer::mapper`)

```rust
pub enum SQLToken<'a> {             // Debug + PartialEq
    Keyword(Keyword),               // Select, From, Insert, Into, Values, Where
    Punctuation(Punctuation),       // Star, Comma, Dot, Lparen, Rparen, Equals, <, >, ;
    Identifier(&'a str),            // user_id, table
    StringLiteral(&'a str),         // 'active'
    NumericLiteral(&'a str),        // 42
    Undefined(&'a str),             // scanned but not yet classified
}
```

Keyword matching is case-insensitive (`eq_ignore_ascii_case`). The distinct
`'a` lifetime on tokens keeps them borrowing from the original input —
tokenizing never allocates or copies strings.

## SQL CFG/PDA

To start, it makes sense to define the possible structures a SQL query can look
like and break down the possible expressions/tokens that comprise it. In other
words, we need a Context Free Grammar / Push Down Automaton that defines the
valid sequences of SQL tokens.

Conventions: `::=` defines a rule, `|` alternatives, `[ ... ]` optional,
`"..."/'...'` literal tokens. Terminal tokens are handled by the lexer; the
parser never sees characters, only the token stream.

```ebnf
(* Statements *)
<Statement>     ::= <SelectStmt> | <InsertStmt>

(* SELECT *)
<SelectStmt>    ::= "SELECT" <SelectList> "FROM" <TableSource>
                    [ "WHERE" <Predicate> ]
                    [ "ORDER BY" <OrderList> ]
                    [ "LIMIT" <NumericLiteral> ]
<SelectList>    ::= "*" | <SelectItem> | <SelectItem> "," <SelectList>
<SelectItem>    ::= <ColumnRef> [ "AS" <Identifier> ]
<ColumnRef>     ::= <Identifier> | <Identifier> "." <Identifier>
<TableSource>   ::= <TableName> | "(" <SelectStmt> ")"
<TableName>     ::= <Identifier>
<OrderList>     ::= <OrderItem> | <OrderItem> "," <OrderList>
<OrderItem>     ::= <ColumnRef> [ "ASC" | "DESC" ]

(* WHERE — precedence layering, tightest binds last *)
<Predicate>     ::= <OrExpr>
<OrExpr>        ::= <AndExpr> | <OrExpr> "OR" <AndExpr>
<AndExpr>       ::= <NotExpr> | <AndExpr> "AND" <NotExpr>
<NotExpr>       ::= <Comparison> | "NOT" <NotExpr>
<Comparison>    ::= <Expr> <CompareOp> <Expr> | "(" <Predicate> ")"
<CompareOp>     ::= "=" | "<>" | "!=" | "<" | "<=" | ">" | ">="
<Expr>          ::= <ColumnRef> | <StringLiteral> | <NumericLiteral>

(* INSERT *)
<InsertStmt>    ::= "INSERT INTO" <TableName> [ "(" <ColumnList> ")" ]
                    "VALUES" <TupleList>
<ColumnList>    ::= <Identifier> | <Identifier> "," <ColumnList>
<TupleList>     ::= <Tuple> | <Tuple> "," <TupleList>
<Tuple>         ::= "(" <ValueList> ")"
<ValueList>     ::= <Value> | <Value> "," <ValueList>
<Value>         ::= <StringLiteral> | <NumericLiteral>

(* Terminal Tokens (Handled by Tokenizer/Lexer) *)
<Identifier>      ::= [a-zA-Z_][a-zA-Z0-9_]*
<StringLiteral>   ::= STRING_LITERAL
<NumericLiteral>  ::= NUMBER_LITERAL
```

## Current Status

Implemented and working:

- [x] `Scanner`: `peek`, `advance`, `next_segment`, `consume_segment`
- [x] Position tracking (byte offset / line / column), UTF-8 multi-byte aware
- [x] Whitespace skipping: whitespace is consumed, never emitted
- [x] Two scan-error rules: unterminated quoted string, and digit-led
      identifiers (`123abc` is invalid — identifiers can't start with a digit)
- [x] `Segment` + `RawKind` data model
- [x] `token_mapper` classifies over `RawKind` (keywords case-insensitive via
      `eq_ignore_ascii_case`)
- [x] Word fallback in the mapper: unknown word → `Identifier` instead of
      `Undefined`; unknown symbols stay `Undefined`
- [x] Literal split: `StringLiteral` vs `NumericLiteral`
- [x] `tokenize(scanner)` drive loop (replaces the naive `split(' ')`)
- [x] `ScanError` propagated through `next_segment` → `tokenize`
- [x] lib + bin split; 25 unit tests + 8 integration tests, all passing (`cargo test`)

Decided and designed (documented in code as future work):

- [ ] Multi-char operators (`<=`, `>=`, `!=`, `<>`) need a scanner max-munch rule
      before the mapper can recognize them
- [ ] `Number` classification beyond raw run (float / sign), and whether to strip
      quotes from string literals
- [ ] `ScanError` should carry `line`/`col` position data (fields are ready on
      `Segment` for this)
- [ ] Decide ASCII vs Unicode word characters (`is_alphanumeric` vs
      `is_ascii_alphanumeric`) — currently Unicode-aware

## Module Layout

```
src/
├── lib.rs               # crate root: pub mod errors; pub mod lexer;
├── main.rs              # thin bin: wires Scanner → tokenize → print
├── errors/
│   └── errors.rs        # ScanError (error layer for the lexer)
└── lexer/
    ├── mod.rs
    ├── mapper.rs        # token_mapper + tokenize drive loop + SQLToken
    └── scanner.rs       # Scanner, Segment, RawKind
tests/
└── tokenizer.rs         # integration tests (public API, black-box)
```

## Testing

```sh
cargo test    # 25 unit tests + 8 integration tests
cargo run     # tokenize the sample query and print the token stream
```

## License

MIT — see [LICENSE](LICENSE).
