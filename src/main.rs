mod errors;
mod lexer;

use crate::lexer::mapper::{tokenize, SQLToken};
use crate::lexer::scanner::Scanner;

fn main() {
    let query = "SELECT user_id , temperature FROM table WHERE status = 'active'";

    let mut scanner = Scanner::new(query);
    let tokens: Vec<SQLToken<'_>> = tokenize(&mut scanner);

    for token in tokens {
        println!("{:?}", token);
    }
}