use verrost::satisfies;

fn main() {
    match satisfies("1.5.0", "^1.2.3") {
        Ok(result) => println!("Matches: {}", result),
        Err(err) => eprintln!("Error {:?}", err),
    }
}
