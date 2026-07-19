#[derive(Debug, PartialEq)]
pub enum Comparator {
    Eq,
    Gt,
    Lt,
    Gte,
    Lte,
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Text(&'a str),
    Number(&'a str),
    Dot,
    Tilde,
    Caret,
    Wildcard,
    Dash,
    Plus,
    Whitespace(&'a str),
    LogicalOr,
    Comparator(Comparator),
    Eof,

    Unknown(&'a str),
}

pub struct Lexer {
    position: usize,
}

impl Lexer {
    pub fn new() -> Self {
        Self { position: 0 }
    }

    fn peek<'a>(&self, input: &'a str) -> Option<char> {
        let mut chars = input[self.position..].chars();

        return chars.next();
    }

    fn advance<'a>(&mut self, input: &'a str) -> Option<char> {
        let c = self.peek(input)?;

        self.position += c.len_utf8();

        Some(c)
    }

    fn consume_while<'a, F>(&mut self, input: &'a str, condition: F) -> &'a str
    where
        F: Fn(char) -> bool,
    {
        let start = self.position;

        while let Some(c) = self.peek(input) {
            if !condition(c) {
                break;
            }

            self.advance(input);
        }

        return &input[start..self.position];
    }

    fn matches(&mut self, input: &str, expected: char) -> bool {
        if (self.peek(input)) == Some(expected) {
            self.advance(input);
            true
        } else {
            false
        }
    }

    pub fn parse<'a>(&mut self, input: &'a str) -> Vec<Token<'a>> {
        self.position = 0;
        let mut tokens = Vec::new();

        while self.position < input.len() {
            let c = self.peek(input).unwrap();

            match c {
                c if c.is_ascii_digit() => {
                    let number = self.consume_while(input, |c| c.is_ascii_digit());

                    tokens.push(Token::Number(number));
                }

                c if c.is_alphabetic() => {
                    let text = self.consume_while(input, |c| {
                        c.is_alphabetic() || matches!(c, '-' | ':' | '/')
                    });

                    tokens.push(Token::Text(text));
                }

                // TODO: should use c.is_ascii_whitespace() instead?
                c if c.is_whitespace() => {
                    let spaces = self.consume_while(input, |c| c.is_whitespace());

                    tokens.push(Token::Whitespace(spaces));
                }

                '.' => {
                    tokens.push(Token::Dot);

                    self.advance(input);
                }

                '~' => {
                    tokens.push(Token::Tilde);

                    self.advance(input);
                }

                '^' => {
                    tokens.push(Token::Caret);

                    self.advance(input);
                }

                '*' => {
                    tokens.push(Token::Wildcard);

                    self.advance(input);
                }

                '-' => {
                    tokens.push(Token::Dash);

                    self.advance(input);
                }

                '+' => {
                    tokens.push(Token::Plus);

                    self.advance(input);
                }

                '=' => {
                    tokens.push(Token::Comparator(Comparator::Eq));

                    self.advance(input);
                }

                '|' => {
                    let start = self.position;
                    self.advance(input);

                    if self.matches(input, '|') {
                        tokens.push(Token::LogicalOr);
                    } else {
                        tokens.push(Token::Unknown(&input[start..self.position]));
                    }
                }

                '>' => {
                    self.advance(input);

                    let condition = if self.matches(input, '=') {
                        Comparator::Gte
                    } else {
                        Comparator::Gt
                    };

                    tokens.push(Token::Comparator(condition));
                }

                '<' => {
                    self.advance(input);

                    let condition = if self.matches(input, '=') {
                        Comparator::Lte
                    } else {
                        Comparator::Lt
                    };

                    tokens.push(Token::Comparator(condition));
                }

                _ => {
                    tokens.push(Token::Unknown(&input[self.position..self.position + 1]));

                    self.advance(input);
                }
            }
        }

        tokens.push(Token::Eof);

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn format_token_diff(actual: &[Token], expected: &[Token]) -> String {
        let mut output = String::new();

        output.push_str("Idx | Actual           | Expected\n");
        output.push_str("----+------------------+------------------\n");

        let max_len = actual.len().max(expected.len());

        for i in 0..max_len {
            let actual_token = actual
                .get(i)
                .map(|t| format!("{:?}", t))
                .unwrap_or_default();

            let expected_token = expected
                .get(i)
                .map(|t| format!("{:?}", t))
                .unwrap_or_default();

            output.push_str(&format!(
                "{:<3} | {:<16} | {:<16}\n",
                i, actual_token, expected_token
            ));
        }

        output
    }

    #[rstest]
    // --- Basic Semantic Versions ---
    #[case(
        "1.2.3",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "v1.2.3",
        vec![
            Token::Text("v"),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "10.20.30",
        vec![
            Token::Number("10"),
            Token::Dot,
            Token::Number("20"),
            Token::Dot,
            Token::Number("30"),
            Token::Eof,
        ]
    )]
    #[case(
        "1",
        vec![
            Token::Number("1"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.2",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Eof,
        ]
    )]
    // --- Equal ---
    #[case(
        "=",
        vec![
            Token::Comparator(Comparator::Eq),
            Token::Eof,
        ]
    )]
    #[case(
        "=1.2.3",
        vec![
            Token::Comparator(Comparator::Eq),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "= 1.2.3",
        vec![
            Token::Comparator(Comparator::Eq),
            Token::Whitespace(" "),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "=1.2.3 || =2.0.0",
        vec![
            Token::Comparator(Comparator::Eq),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Whitespace(" "),
            Token::LogicalOr,
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Eq),
            Token::Number("2"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    // --- Range Operators ---
    #[case(
        "^1.2.3",
        vec![
            Token::Caret,
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "~1.2.3",
        vec![
            Token::Tilde,
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        ">=1.2.3",
        vec![
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "<=1.2.3",
        vec![
            Token::Comparator(Comparator::Lte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        ">1.2.3",
        vec![
            Token::Comparator(Comparator::Gt),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "<1.2.3",
        vec![
            Token::Comparator(Comparator::Lt),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    // --- Logical OR ---
    #[case(
        "||",
        vec![
            Token::LogicalOr,
            Token::Eof,
        ]
    )]
    #[case(
        "1.2.3 || 2.0.0",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Whitespace(" "),
            Token::LogicalOr,
            Token::Whitespace(" "),
            Token::Number("2"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    // --- Logical AND ---
    #[case(
        ">=1.2.3 <2.0.0",
        vec![
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Lt),
            Token::Number("2"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    #[case(
        ">=1.2.3-0 <2.0.0",
        vec![
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Dash,
            Token::Number("0"),
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Lt),
            Token::Number("2"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.x >=1.2.0",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Text("x"),
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    #[case(
        "~1.2 >=1.2.3",
        vec![
            Token::Tilde,
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        ">=1.0.0 <2.0.0 >=1.5.0",
        vec![
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Lt),
            Token::Number("2"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("5"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    #[case(
        ">=1.0.0 <2.0.0 || >=3.0.0",
        vec![
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Lt),
            Token::Number("2"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Whitespace(" "),
            Token::LogicalOr,
            Token::Whitespace(" "),
            Token::Comparator(Comparator::Gte),
            Token::Number("3"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    // --- Pre-release and Build Metadata ---
    #[case(
        "1.0.0-alpha.1",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dash,
            Token::Text("alpha"),
            Token::Dot,
            Token::Number("1"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.0.0-alpha.beta",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dash,
            Token::Text("alpha"),
            Token::Dot,
            Token::Text("beta"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.0.0-0.3.7",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dash,
            Token::Number("0"),
            Token::Dot,
            Token::Number("3"),
            Token::Dot,
            Token::Number("7"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.0.0-x.7.z.92",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dash,
            Token::Text("x"),
            Token::Dot,
            Token::Number("7"),
            Token::Dot,
            Token::Text("z"),
            Token::Dot,
            Token::Number("92"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.0.0+build.1",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Plus,
            Token::Text("build"),
            Token::Dot,
            Token::Number("1"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.2.3-beta.4+build.5678",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Dash,
            Token::Text("beta"),
            Token::Dot,
            Token::Number("4"),
            Token::Plus,
            Token::Text("build"),
            Token::Dot,
            Token::Number("5678"),
            Token::Eof,
        ]
    )]
    #[case(
    ">=1.2.3-alpha || <=2.0.0+build.42",
    vec![
        Token::Comparator(Comparator::Gte),
        Token::Number("1"),
        Token::Dot,
        Token::Number("2"),
        Token::Dot,
        Token::Number("3"),
        Token::Dash,
        Token::Text("alpha"),
        Token::Whitespace(" "),
        Token::LogicalOr,
        Token::Whitespace(" "),
        Token::Comparator(Comparator::Lte),
        Token::Number("2"),
        Token::Dot,
        Token::Number("0"),
        Token::Dot,
        Token::Number("0"),
        Token::Plus,
        Token::Text("build"),
        Token::Dot,
        Token::Number("42"),
        Token::Eof,
    ]
)]
    // --- Whitespace Handling ---
    #[case(
        ">= 1.2.3",
        vec![
            Token::Comparator(Comparator::Gte),
            Token::Whitespace(" "),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.2.3   ||   4.5.6",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Whitespace("   "),
            Token::LogicalOr,
            Token::Whitespace("   "),
            Token::Number("4"),
            Token::Dot,
            Token::Number("5"),
            Token::Dot,
            Token::Number("6"),
            Token::Eof,
        ]
    )]
    #[case(
        "   1.2.3   ",
        vec![
            Token::Whitespace("   "),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Whitespace("   "),
            Token::Eof,
        ]
    )]
    // --- Wildcards ---
    #[case(
        "x",
        vec![
            Token::Text("x"),
            Token::Eof,
        ]
    )]
    #[case(
        "X",
        vec![
            Token::Text("X"),
            Token::Eof,
        ]
    )]
    #[case(
        "*",
        vec![
            Token::Wildcard,
            Token::Eof,
        ]
    )]
    #[case(
        "1.x",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Text("x"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.X",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Text("X"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.*",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Wildcard,
            Token::Eof,
        ]
    )]
    #[case(
        "1.x.x",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Text("x"),
            Token::Dot,
            Token::Text("x"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.X.*",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Text("X"),
            Token::Dot,
            Token::Wildcard,
            Token::Eof,
        ]
    )]
    #[case(
        "x.x.x",
        vec![
            Token::Text("x"),
            Token::Dot,
            Token::Text("x"),
            Token::Dot,
            Token::Text("x"),
            Token::Eof,
        ]
    )]
    #[case(
        "^1.x",
        vec![
            Token::Caret,
            Token::Number("1"),
            Token::Dot,
            Token::Text("x"),
            Token::Eof,
        ]
    )]
    // --- Hypen Ranges ---
    #[case(
        "1.2.3 - 2.3.4",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Whitespace(" "),
            Token::Dash,
            Token::Whitespace(" "),
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Dot,
            Token::Number("4"),
            Token::Eof,
        ]
    )]
    // --- Interesting Edge Cases ---
    #[case(
        "^0.0.3",
        vec![
            Token::Caret,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    #[case(
        "0.0.0-0",
        vec![
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dash,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    #[case(
        ">=1.2.3-0",
        vec![
            Token::Comparator(Comparator::Gte),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Dash,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    // --- Url, Aliases ---
    #[case(
        "git://github.com/user/project.git",
        vec![
            Token::Text("git://github"),
            Token::Dot,
            Token::Text("com/user/project"),
            Token::Dot,
            Token::Text("git"),
            Token::Eof,
        ]
    )]
    #[case(
        "file:../local-pkg",
        vec![
            Token::Text("file:"),
            Token::Dot,
            Token::Dot,
            Token::Unknown("/"),
            Token::Text("local-pkg"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.0.0-alpha-beta",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dash,
            Token::Text("alpha-beta"),
            Token::Eof,
        ]
    )]
    #[case(
        "1.0.0-x-y-z",
        vec![
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Dash,
            Token::Text("x-y-z"),
            Token::Eof,
        ]
    )]
    #[case(
        "github:user/repo#v1.0.0",
        vec![
            Token::Text("github:user/repo"),
            Token::Unknown("#"),
            Token::Text("v"),
            Token::Number("1"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    #[case(
        "npm:lodash@^3.0.0",
        vec![
            Token::Text("npm:lodash"),
            Token::Unknown("@"),
            Token::Caret,
            Token::Number("3"),
            Token::Dot,
            Token::Number("0"),
            Token::Dot,
            Token::Number("0"),
            Token::Eof,
        ]
    )]
    #[case(
        "npm:@org/pkg@1.2.3",
        vec![
            Token::Text("npm:"),
            Token::Unknown("@"),
            Token::Text("org/pkg"),
            Token::Unknown("@"),
            Token::Number("1"),
            Token::Dot,
            Token::Number("2"),
            Token::Dot,
            Token::Number("3"),
            Token::Eof,
        ]
    )]
    fn test_lexer_cases(#[case] input: &str, #[case] expected: Vec<Token>) {
        let mut lexer = Lexer::new();
        let actual = lexer.parse(input);

        assert!(
            actual == expected,
            "Token mismatch for input {:?}\n\n{}",
            input,
            format_token_diff(&actual, &expected)
        );
    }
}
