use goose_db::lexer::mapper::{tokenize, Keyword, Punctuation, SQLToken};
use goose_db::lexer::scanner::Scanner;

#[test]
fn tokenizes_whole_select_query() {
    let query = "SELECT user_id , temperature FROM table WHERE status = 'active'";

    let mut scanner = Scanner::new(query);
    let tokens = tokenize(&mut scanner).unwrap();

    assert_eq!(
        tokens,
        vec![
            SQLToken::Keyword(Keyword::Select),
            SQLToken::Identifier("user_id"),
            SQLToken::Punctuation(Punctuation::Comma),
            SQLToken::Identifier("temperature"),
            SQLToken::Keyword(Keyword::From),
            SQLToken::Identifier("table"),
            SQLToken::Keyword(Keyword::Where),
            SQLToken::Identifier("status"),
            SQLToken::Punctuation(Punctuation::Equals),
            SQLToken::StringLiteral("'active'"),
        ]
    );
}

#[test]
fn tokenize_of_empty_input_is_empty() {
    let mut scanner = Scanner::new("");
    assert!(tokenize(&mut scanner).unwrap().is_empty());
}

#[test]
fn tokenize_of_whitespace_only_input_is_empty() {
    let mut scanner = Scanner::new("   \n\t  ");
    assert!(tokenize(&mut scanner).unwrap().is_empty());
}

#[test]
fn tokenize_propagates_unterminated_string_error() {
    let mut scanner = Scanner::new("SELECT 'oops");
    assert!(tokenize(&mut scanner).is_err());
}

#[test]
fn tokenize_skips_whitespace_between_tokens() {
    let mut scanner = Scanner::new("a  ,\n b");
    let tokens = tokenize(&mut scanner).unwrap();
    assert_eq!(
        tokens,
        vec![
            SQLToken::Identifier("a"),
            SQLToken::Punctuation(Punctuation::Comma),
            SQLToken::Identifier("b"),
        ]
    );
}

#[test]
fn tokenize_rejects_digit_led_identifiers() {
    let mut scanner = Scanner::new("SELECT 123abc FROM t");
    assert!(tokenize(&mut scanner).is_err());
}

#[test]
fn tokenize_accepts_number_followed_by_symbol() {
    let mut scanner = Scanner::new("SELECT 123, FROM t");
    assert!(tokenize(&mut scanner).is_ok());
}
    #[test]
fn tokenize_is_deterministic_across_identical_scanners() {
    let query = "INSERT INTO events VALUES (1, 'x')";
    let tokens_a = tokenize(&mut Scanner::new(query)).unwrap();
    let tokens_b = tokenize(&mut Scanner::new(query)).unwrap();
    assert_eq!(tokens_a, tokens_b);
}