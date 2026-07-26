use verrost::satisfies;

fn main() {
    match satisfies("1.2.3-alpha+build", ">=1.2.3 <2.0.0") {
        Ok(result) => println!("Matches: {}", result),
        Err(err) => eprintln!("Error {:?}", err),
    }
}
