use goose_db::lexer::mapper::tokenize;
use goose_db::lexer::scanner::Scanner;

fn main() {
    let query = "SELECT user_id , temperature FROM table WHERE status = 'active'";

    let mut scanner = Scanner::new(query);
    match tokenize(&mut scanner) {
        Ok(tokens) => {
            for token in tokens {
                println!("{:?}", token);
            }
        }
        Err(e) => {
            eprintln!("tokenizer error: {e}");
            std::process::exit(1);
        }
    }
}
