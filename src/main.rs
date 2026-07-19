use verrost::lexer::Lexer;

fn main() {
    let mut lexer = Lexer::new();
    let tokens = lexer.parse("1.2.3");

    println!("Hello, world! {:?}", tokens);
}
