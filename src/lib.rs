use crate::{
    lexer::Lexer,
    parser::{Parser, ParserError, Version, VersionRange},
};

pub mod lexer;
pub mod parser;
pub mod satisfies;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    InvalidVersion(ParserError),
    InvalidRange(ParserError),
}

pub fn satisfies(version_str: &str, range_str: &str) -> Result<bool, Error> {
    let range = parse_range(range_str).map_err(Error::InvalidRange)?;

    let version = parse_version(version_str).map_err(Error::InvalidVersion)?;

    // println!("{:#?}", range);

    Ok(range.satisfies(&version))
}

pub fn parse_range(input: &str) -> Result<VersionRange<'_>, ParserError> {
    let mut lexer = Lexer::new();
    let tokens = lexer.parse(input);

    let mut parser = Parser::new(tokens);

    parser.parse()
}

pub fn parse_version(input: &str) -> Result<Version<'_>, ParserError> {
    let mut lexer = Lexer::new();
    let tokens = lexer.parse(input);

    let mut parser = Parser::new(tokens);

    parser.parse_version()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // -------------------------------------------------------------------------
    // parse_version
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("1.2.3", true)]
    #[case("1.2.3-alpha", true)]
    #[case("1.2.3+build", true)]
    #[case("1.2.3-alpha+build", true)]
    #[case("1", false)]
    #[case("1.2", false)]
    #[case("1.x", false)]
    #[case("^1.2.3", false)]
    fn parse_version_api(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(parse_version(input).is_ok(), expected);
    }

    // -------------------------------------------------------------------------
    // parse_range
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("1.2.3", true)]
    #[case("^1.2.3", true)]
    #[case("~1.2", true)]
    #[case(">=1.2.3 <2.0.0", true)]
    #[case("1.x", true)]
    #[case("*", true)]
    #[case("", false)]
    fn parse_range_api(#[case] input: &str, #[case] expected: bool) {
        assert_eq!(parse_range(input).is_ok(), expected);
    }

    // -------------------------------------------------------------------------
    // satisfies
    // -------------------------------------------------------------------------

    #[rstest]
    // equality
    #[case("1.2.3", "1.2.3", true)]
    #[case("1.2.4", "1.2.3", false)]
    // comparison operators
    #[case("1.2.4", ">1.2.3", true)]
    #[case("1.2.3", ">1.2.3", false)]
    #[case("1.2.3", ">=1.2.3", true)]
    #[case("1.2.2", ">=1.2.3", false)]
    #[case("1.2.2", "<1.2.3", true)]
    #[case("1.2.3", "<1.2.3", false)]
    #[case("1.2.3", "<=1.2.3", true)]
    #[case("1.2.4", "<=1.2.3", false)]
    // caret
    #[case("1.5.0", "^1.2.3", true)]
    #[case("2.0.0", "^1.2.3", false)]
    // tilde
    #[case("1.2.9", "~1.2.3", true)]
    #[case("1.3.0", "~1.2.3", false)]
    // wildcard
    #[case("1.5.7", "1.x", true)]
    #[case("2.0.0", "1.x", false)]
    // AND
    #[case("1.5.0", ">=1.2.3 <2.0.0", true)]
    #[case("2.0.0", ">=1.2.3 <2.0.0", false)]
    // OR
    #[case("1.5.0", "<2.0.0 || >=3.0.0", true)]
    #[case("3.1.0", "<2.0.0 || >=3.0.0", true)]
    #[case("2.5.0", "<2.0.0 || >=3.0.0", false)]
    fn satisfies_api(#[case] version: &str, #[case] range: &str, #[case] expected: bool) {
        assert_eq!(satisfies(version, range), Ok(expected));
    }

    // -------------------------------------------------------------------------
    // Error propagation
    // -------------------------------------------------------------------------

    #[rstest]
    #[case("1", "1.2.3")]
    #[case("1.x", "1.2.3")]
    #[case("^1.2.3", "1.2.3")]
    fn satisfies_invalid_version(#[case] version: &str, #[case] range: &str) {
        assert!(satisfies(version, range).is_err());
    }

    #[rstest]
    #[case("1.2.3", "")]
    fn satisfies_invalid_range(#[case] version: &str, #[case] range: &str) {
        assert!(satisfies(version, range).is_err());
    }
}
