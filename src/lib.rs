use crate::{lexer::Lexer, parser::Parser};

pub mod lexer;
pub mod parser;

// TODO: add error types later
#[derive(Debug)]
pub enum Error {}

pub fn satisfies(version: &str, range: &str) -> Result<bool, Error> {
    let mut lexer = Lexer::new();
    let tokens = lexer.parse(version);
    let mut parser = Parser::new(tokens);

    let result = parser.parse();

    println!("{:#?}", result);

    Ok(false)
}
