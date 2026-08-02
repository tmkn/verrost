use std::borrow::Cow;

use crate::lexer::Comparator as LexerComparator;
use crate::lexer::Token::{self};

/// A version component that may be a numeric value or a wildcard.
/// Used for major, minor, and patch components.
#[derive(Debug, PartialEq, Eq)]
pub enum VersionComponent {
    Number(u32),
    Wildcard,
}

/// A SemVer identifier used in pre-release and build metadata.
/// Identifiers can be numeric or alphanumeric.
#[derive(Debug, PartialEq, Eq)]
pub enum Identifier<'a> {
    Number(u32),
    Text(Cow<'a, str>),
}

/// A version that hasn't been desugared yet, contains wildcards
#[derive(Debug, PartialEq, Eq)]
pub struct PartialVersion<'a> {
    major: VersionComponent,
    minor: VersionComponent,
    patch: VersionComponent,

    pre_release: Vec<Identifier<'a>>,
    build: Vec<Identifier<'a>>,
}

/// A version with concrete major, minor and patch values
#[derive(Debug, PartialEq, Eq)]
pub struct Version<'a> {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,

    pub pre_release: Vec<Identifier<'a>>,
    pub build: Vec<Identifier<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PartialComparatorOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
    Caret,
    Tilde,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PartialComparator<'a> {
    pub op: PartialComparatorOp,
    pub version: PartialVersion<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ComparatorOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
}

impl TryFrom<PartialComparatorOp> for ComparatorOp {
    type Error = ();

    fn try_from(op: PartialComparatorOp) -> Result<Self, Self::Error> {
        match op {
            PartialComparatorOp::Eq => Ok(ComparatorOp::Eq),
            PartialComparatorOp::Gt => Ok(ComparatorOp::Gt),
            PartialComparatorOp::Gte => Ok(ComparatorOp::Gte),
            PartialComparatorOp::Lt => Ok(ComparatorOp::Lt),
            PartialComparatorOp::Lte => Ok(ComparatorOp::Lte),

            PartialComparatorOp::Caret => Err(()),
            PartialComparatorOp::Tilde => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Comparator<'a> {
    pub op: ComparatorOp,
    pub version: Version<'a>,
}

/// A group of comparators that are combined with logical AND.
///
/// A version must satisfy every comparator in the set for the set to match.
/// Example:
/// `>=1.2.3 <2.0.0`
#[derive(Debug, PartialEq, Eq)]
pub struct ComparatorSet<'a> {
    pub comparators: Vec<Comparator<'a>>,
}

/// A collection of comparator sets combined with logical OR.
///
/// A version must satisfy at least one comparator set for the range to match.
/// Example:
/// `1.2.3 || ^3.1.4`
#[derive(Debug, PartialEq, Eq)]
pub struct VersionRange<'a> {
    pub sets: Vec<ComparatorSet<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
enum ParseModeMetadata {
    PreRelease,
    Build,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParserError {
    Generic,
    InvalidComparatorOp,
    ExpectedVersionComponent,
    InvalidNumericIdentifier,
    InvalidMetadataIdentifier,
    NumericIdentifierLeadingZero,
    ExpectedCompleteVersion,
    UnexpectedToken,
}

pub struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    position: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token<'a>>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn peek_n(&self, n: usize) -> Option<&Token<'a>> {
        self.tokens.get(self.position + n)
    }

    fn peek(&self) -> Option<&Token<'a>> {
        self.peek_n(0)
    }

    fn advance(&mut self) -> Option<&Token<'a>> {
        let token = self.tokens.get(self.position)?;

        self.position += 1;

        Some(token)
    }

    pub fn parse(&mut self) -> Result<VersionRange<'a>, ParserError> {
        // while let Some(token) = self.advance() {
        //     match token {
        //         _ => {
        //             println!("{:?}", token);
        //         }
        //     }
        // }

        self.position = 0;

        self.parse_version_range()
    }

    fn try_consume_version_range(&mut self) -> bool {
        if let Some(&Token::Whitespace(_)) = self.peek() {
            self.advance();
        }

        if !matches!(self.peek(), Some(&Token::LogicalOr)) {
            return false;
        }

        self.advance();
        if let Some(&Token::Whitespace(_)) = self.peek() {
            self.advance();
        }

        true
    }

    pub fn parse_version_range(&mut self) -> Result<VersionRange<'a>, ParserError> {
        let mut sets: Vec<ComparatorSet<'a>> = Vec::new();

        let comparator_set = self.parse_comparator_set()?;

        sets.push(comparator_set);

        while self.try_consume_version_range() {
            let comparator_set = self.parse_comparator_set()?;

            sets.push(comparator_set);
        }

        Ok(VersionRange { sets })
    }

    pub fn parse_comparator_set(&mut self) -> Result<ComparatorSet<'a>, ParserError> {
        let mut comparators: Vec<Comparator<'a>> = vec![];

        let partial_comparator = self.parse_partial_comparator()?;
        let desugared_comparator = self.desugar_partial_comparator(partial_comparator)?;

        comparators.extend(desugared_comparator);

        while matches!(self.peek(), Some(Token::Whitespace(_)))
            && !matches!(self.peek_n(1), Some(Token::LogicalOr))
        {
            self.advance();

            let partial_comparator = self.parse_partial_comparator()?;
            let desugared_comparator = self.desugar_partial_comparator(partial_comparator)?;

            comparators.extend(desugared_comparator);
        }

        Ok(ComparatorSet { comparators })
    }

    fn parse_partial_comparator_op(&mut self) -> Option<PartialComparatorOp> {
        match self.peek() {
            Some(Token::Caret) => {
                self.advance();
                Some(PartialComparatorOp::Caret)
            }
            Some(Token::Tilde) => {
                self.advance();
                Some(PartialComparatorOp::Tilde)
            }
            Some(Token::Comparator(LexerComparator::Eq)) => {
                self.advance();
                Some(PartialComparatorOp::Eq)
            }
            Some(Token::Comparator(LexerComparator::Lt)) => {
                self.advance();
                Some(PartialComparatorOp::Lt)
            }
            Some(Token::Comparator(LexerComparator::Lte)) => {
                self.advance();
                Some(PartialComparatorOp::Lte)
            }
            Some(Token::Comparator(LexerComparator::Gt)) => {
                self.advance();
                Some(PartialComparatorOp::Gt)
            }
            Some(Token::Comparator(LexerComparator::Gte)) => {
                self.advance();
                Some(PartialComparatorOp::Gte)
            }
            _ => None,
        }
    }

    pub fn parse_partial_comparator(&mut self) -> Result<PartialComparator<'a>, ParserError> {
        let partial_comparator_op = self
            .parse_partial_comparator_op()
            .unwrap_or(PartialComparatorOp::Eq);
        let partial_version = self.parse_partial_version()?;

        Ok(PartialComparator {
            op: partial_comparator_op,
            version: partial_version,
        })
    }

    pub fn parse_partial_version(&mut self) -> Result<PartialVersion<'a>, ParserError> {
        let major = match self.peek() {
            Some(Token::Number(_)) => self.parse_number_token().ok_or(ParserError::Generic)?,
            Some(Token::Wildcard) => {
                self.advance();
                return Ok(PartialVersion {
                    major: VersionComponent::Wildcard,
                    minor: VersionComponent::Wildcard,
                    patch: VersionComponent::Wildcard,
                    pre_release: vec![],
                    build: vec![],
                });
            }
            _ => return Err(ParserError::ExpectedVersionComponent),
        };
        let minor: Option<u32> = match self.peek() {
            Some(Token::Dot) => {
                self.advance();
                self.parse_number_token()
            }
            _ => None,
        };
        let patch: Option<u32> = match self.peek() {
            Some(Token::Dot) => {
                self.advance();
                self.parse_number_token()
            }
            _ => None,
        };

        let mut pre_release: Vec<Identifier<'a>> = Vec::new();
        let mut build: Vec<Identifier<'a>> = Vec::new();

        if let Some(_) = patch {
            // A hyphen introduces pre-release identifiers.
            if let Some(Token::Dash) = self.peek() {
                self.advance();

                pre_release.extend(self.parse_metadata(ParseModeMetadata::PreRelease)?);
            }

            // A plus sign introduces build metadata identifiers.
            if let Some(Token::Plus) = self.peek() {
                self.advance();

                build.extend(self.parse_metadata(ParseModeMetadata::Build)?);
            }
        }

        Ok(PartialVersion {
            major: VersionComponent::Number(major),
            minor: match minor {
                Some(value) => VersionComponent::Number(value),
                None => VersionComponent::Wildcard,
            },
            patch: match patch {
                Some(value) => VersionComponent::Number(value),
                None => VersionComponent::Wildcard,
            },
            pre_release,
            build,
        })
    }

    // TODO: Split pre-release and build parsing into dedicated functions.
    // They share identifier parsing, but have different validation rules.
    fn parse_metadata(
        &mut self,
        mode: ParseModeMetadata,
    ) -> Result<Vec<Identifier<'a>>, ParserError> {
        let mut metadata = Vec::new();
        let mut identifier = String::new();

        loop {
            match self.peek() {
                Some(&Token::Number(text)) => {
                    self.advance();
                    identifier.push_str(text);
                }

                Some(&Token::Text(text)) => {
                    self.advance();
                    identifier.push_str(text);
                }

                // A '+' terminates pre-release metadata and starts build metadata.
                Some(&Token::Plus) if mode == ParseModeMetadata::PreRelease => {
                    if !identifier.is_empty() {
                        let identifier = std::mem::take(&mut identifier);

                        let part = if identifier.chars().all(|c| c.is_ascii_digit()) {
                            if mode == ParseModeMetadata::PreRelease
                                && has_leading_zero(&identifier)
                            {
                                return Err(ParserError::NumericIdentifierLeadingZero);
                            }

                            Identifier::Number(
                                identifier
                                    .parse::<u32>()
                                    .map_err(|_| ParserError::InvalidNumericIdentifier)?,
                            )
                        } else {
                            Identifier::Text(identifier.into())
                        };

                        metadata.push(part);

                        return Ok(metadata);
                    }
                }

                // Dots separate metadata identifiers but do not end metadata parsing.
                // For example: "rc.1" becomes [Text("rc"), Number(1)].
                Some(Token::Dot) | Some(Token::Eof) => {
                    if !identifier.is_empty() {
                        let identifier = std::mem::take(&mut identifier);

                        let part = if identifier.chars().all(|c| c.is_ascii_digit()) {
                            if mode == ParseModeMetadata::PreRelease
                                && has_leading_zero(&identifier)
                            {
                                return Err(ParserError::NumericIdentifierLeadingZero);
                            }

                            Identifier::Number(
                                identifier
                                    .parse::<u32>()
                                    .map_err(|_| ParserError::InvalidNumericIdentifier)?,
                            )
                        } else {
                            Identifier::Text(identifier.into())
                        };

                        metadata.push(part);
                    }

                    match self.peek() {
                        Some(Token::Dot) => {
                            self.advance();
                        }

                        Some(Token::Eof) => {
                            break;
                        }

                        // Only Dot or Eof can reach this point because the outer match arm
                        // already restricted the input. Any other token indicates a bug in the
                        // parser's control flow rather than invalid user input.
                        _ => return Err(ParserError::UnexpectedToken),
                    }
                }

                // A '+' inside build metadata is invalid because only one build section is allowed.
                Some(&Token::Plus) if mode == ParseModeMetadata::Build => {
                    return Err(ParserError::InvalidMetadataIdentifier);
                }

                _ => break,
            }
        }

        Ok(metadata)
    }

    fn parse_number_token(&mut self) -> Option<u32> {
        match self.advance()? {
            Token::Number(digit) => digit.parse::<u32>().ok(),
            _ => None,
        }
    }

    fn desugar_partial_comparator(
        &self,
        partial_comparator: PartialComparator<'a>,
    ) -> Result<Vec<Comparator<'a>>, ParserError> {
        use VersionComponent::{Number, Wildcard};

        let PartialComparator { op, version } = partial_comparator;
        let PartialVersion {
            major,
            minor,
            patch,
            pre_release,
            build,
        } = version;

        match (op, major, minor, patch) {
            // ^: Allow the latest compatible version without changing the leftmost non-zero component.
            // Pre-1.0.0 caret rules (0.x releases)
            (PartialComparatorOp::Caret, major, minor, patch) => match (major, minor, patch) {
                (Number(0), Wildcard, Wildcard) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 0,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                (Number(0), Number(0), Number(patch)) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 0,
                            minor: 0,
                            patch,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 0,
                            minor: 0,
                            patch: patch + 1,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                (Number(0), Number(minor), Wildcard) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 0,
                            minor,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 0,
                            minor: minor + 1,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                (Number(0), Number(minor), Number(patch)) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 0,
                            minor,
                            patch,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 0,
                            minor: minor + 1,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                // Caret (^) ranges for major versions >= 1
                (Number(major), Wildcard, Wildcard) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: major + 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                (Number(major), Number(minor), Wildcard) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: major + 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                (Number(major), Number(minor), Number(patch)) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor,
                            patch,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: major + 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                _ => Err(ParserError::Generic),
            },
            // ~: Allow the latest compatible patch/minor version.
            (PartialComparatorOp::Tilde, major, minor, patch) => match (major, minor, patch) {
                (Number(major), Wildcard, Wildcard) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: major + 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                (Number(major), Number(minor), Wildcard) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major,
                            minor: minor + 1,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                (Number(major), Number(minor), Number(patch)) => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor,
                            patch,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major,
                            minor: minor + 1,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                _ => Err(ParserError::Generic),
            },
            // "1.2.3"
            (op, Number(major), Number(minor), Number(patch)) => Ok(vec![Comparator {
                op: ComparatorOp::try_from(op).map_err(|_| ParserError::InvalidComparatorOp)?,
                version: Version {
                    major,
                    minor,
                    patch,
                    pre_release,
                    build,
                },
            }]),
            // "*"
            (_, Wildcard, Wildcard, Wildcard) => Ok(vec![Comparator {
                op: ComparatorOp::Gte,
                version: Version {
                    major: 0,
                    minor: 0,
                    patch: 0,
                    pre_release: vec![],
                    build: vec![],
                },
            }]),

            // "1" (e.g., >=1.0.0, <2.0.0)
            (op, Number(major), Wildcard, Wildcard) => match op {
                PartialComparatorOp::Eq => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: major + 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                PartialComparatorOp::Gt => Ok(vec![Comparator {
                    op: ComparatorOp::Gt,
                    version: Version {
                        major,
                        minor: 0,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                PartialComparatorOp::Gte => Ok(vec![Comparator {
                    op: ComparatorOp::Gte,
                    version: Version {
                        major,
                        minor: 0,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                PartialComparatorOp::Lt => Ok(vec![Comparator {
                    op: ComparatorOp::Lt,
                    version: Version {
                        major: major + 1,
                        minor: 0,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                PartialComparatorOp::Lte => Ok(vec![Comparator {
                    op: ComparatorOp::Lt,
                    version: Version {
                        major: major + 1,
                        minor: 0,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                _ => Err(ParserError::InvalidComparatorOp),
            },

            // "1.2" (e.g., >=1.2.0, <1.3.0)
            (op, Number(major), Number(minor), Wildcard) => match op {
                PartialComparatorOp::Eq => Ok(vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major,
                            minor,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major,
                            minor: minor + 1,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ]),
                PartialComparatorOp::Gt => Ok(vec![Comparator {
                    op: ComparatorOp::Gt,
                    version: Version {
                        major,
                        minor,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                PartialComparatorOp::Gte => Ok(vec![Comparator {
                    op: ComparatorOp::Gte,
                    version: Version {
                        major,
                        minor,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                PartialComparatorOp::Lt => Ok(vec![Comparator {
                    op: ComparatorOp::Lt,
                    version: Version {
                        major,
                        minor: minor + 1,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                PartialComparatorOp::Lte => Ok(vec![Comparator {
                    op: ComparatorOp::Lt,
                    version: Version {
                        major,
                        minor: minor + 1,
                        patch: 0,
                        pre_release: vec![],
                        build: vec![],
                    },
                }]),
                _ => Err(ParserError::InvalidComparatorOp),
            },

            _ => Err(ParserError::Generic),
        }
    }

    pub fn parse_version(&mut self) -> Result<Version<'a>, ParserError> {
        let partial = self.parse_partial_version()?;

        match (partial.major, partial.minor, partial.patch) {
            (
                VersionComponent::Number(major),
                VersionComponent::Number(minor),
                VersionComponent::Number(patch),
            ) => Ok(Version {
                major,
                minor,
                patch,
                pre_release: partial.pre_release,
                build: partial.build,
            }),

            _ => Err(ParserError::ExpectedCompleteVersion),
        }
    }
}

fn has_leading_zero(s: &str) -> bool {
    s.starts_with('0') && s.len() > 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use VersionComponent::{Number, Wildcard};
    use rstest::rstest;

    fn format_ast_diff(
        actual: &Result<VersionRange, ParserError>,
        expected: &Result<VersionRange, ParserError>,
    ) -> String {
        format!("Actual:\n{:#?}\n\nExpected:\n{:#?}", actual, expected)
    }

    #[rstest]
    #[case(
        "1.2.3",
        Ok(VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Eq,
                            version: Version {
                                major: 1,
                                minor: 2,
                                patch: 3,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                    ],
                },
            ],
        })
    )]
    #[case(
    "1",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 2,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
    )]
    #[case(
    "1.2",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 1,
                            minor: 3,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // =============================================================================
    // Prerelease and build metadata parsing
    // =============================================================================
    // 1.2.3-alpha
    #[case(
    "1.2.3-alpha",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Eq,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 3,
                            pre_release: vec![
                                Identifier::Text("alpha".into()),
                            ],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // 1.2.3+build
    #[case(
    "1.2.3+build",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Eq,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 3,
                            pre_release: vec![],
                            build: vec![
                                Identifier::Text("build".into()),
                            ],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // 1.2.3-alpha+build
    #[case(
    "1.2.3-alpha+build",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Eq,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 3,
                            pre_release: vec![
                                Identifier::Text("alpha".into()),
                            ],
                            build: vec![
                                Identifier::Text("build".into()),
                            ],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // 1.2.3-rc.1+exp.sha.5114f85
    #[case(
    "1.2.3-rc.1+exp.sha.5114f85",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Eq,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 3,
                            pre_release: vec![
                                Identifier::Text("rc".into()),
                                Identifier::Number(1),
                            ],
                            build: vec![
                                Identifier::Text("exp".into()),
                                Identifier::Text("sha".into()),
                                Identifier::Text("5114f85".into()),
                            ],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // 1.2.3+build-alpha
    // Hyphens are valid inside identifiers.
    #[case(
    "1.2.3+build-alpha",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Eq,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 3,
                            pre_release: vec![],
                            build: vec![
                                Identifier::Text("build-alpha".into()),
                            ],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // 1.2.3+build-alpha.1
    #[case(
    "1.2.3+build-alpha.1",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Eq,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 3,
                            pre_release: vec![],
                            build: vec![
                                Identifier::Text("build-alpha".into()),
                                Identifier::Number(1),
                            ],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // 1.2.3-1
    // Numeric prerelease identifiers are stored as numbers.
    #[case(
    "1.2.3-1",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Eq,
                        version: Version {
                            major: 1,
                            minor: 2,
                            patch: 3,
                            pre_release: vec![
                                Identifier::Number(1),
                            ],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
    )]
    // =============================================================================
    // Invalid prerelease/build metadata
    // =============================================================================
    // Numeric prerelease identifiers cannot contain leading zeroes.
    #[case("1.2.3-001", Err(ParserError::NumericIdentifierLeadingZero))]
    // Build metadata cannot contain '+'.
    #[case("1.2.3+build+foo", Err(ParserError::InvalidMetadataIdentifier))]
    #[case(
        ">=1.2.3 <2.0.0 || ^3.0.0",
        Ok(VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![
                        // >=1.2.3
                        Comparator {
                            op: ComparatorOp::Gte,
                            version: Version {
                                major: 1,
                                minor: 2,
                                patch: 3,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                        // <2.0.0
                        Comparator {
                            op: ComparatorOp::Lt,
                            version: Version {
                                major: 2,
                                minor: 0,
                                patch: 0,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                    ],
                },
                ComparatorSet {
                    comparators: vec![
                        // >=3.0.0 (from ^3.0.0)
                        Comparator {
                            op: ComparatorOp::Gte,
                            version: Version {
                                major: 3,
                                minor: 0,
                                patch: 0,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                        // <4.0.0 (from ^3.0.0)
                        Comparator {
                            op: ComparatorOp::Lt,
                            version: Version {
                                major: 4,
                                minor: 0,
                                patch: 0,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                    ],
                },
            ],
        })
    )]
    #[case(
        "1.0.0 || 2.0.0 || 3.0.0",
        Ok(VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Eq,
                            version: Version { major: 1, minor: 0, patch: 0, pre_release: vec![], build: vec![] },
                        },
                    ],
                },
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Eq,
                            version: Version { major: 2, minor: 0, patch: 0, pre_release: vec![], build: vec![] },
                        },
                    ],
                },
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Eq,
                            version: Version { major: 3, minor: 0, patch: 0, pre_release: vec![], build: vec![] },
                        },
                    ],
                },
            ],
        })
    )]
    #[case(
        "1.2||2.3",
        Ok(VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Gte,
                            version: Version { major: 1, minor: 2, patch: 0, pre_release: vec![], build: vec![] },
                        },
                        Comparator {
                            op: ComparatorOp::Lt,
                            version: Version { major: 1, minor: 3, patch: 0, pre_release: vec![], build: vec![] },
                        },
                    ],
                },
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Gte,
                            version: Version { major: 2, minor: 3, patch: 0, pre_release: vec![], build: vec![] },
                        },
                        Comparator {
                            op: ComparatorOp::Lt,
                            version: Version { major: 2, minor: 4, patch: 0, pre_release: vec![], build: vec![] },
                        },
                    ],
                },
            ],
        })
    )]
    #[case("1.2.3 ||", Err(ParserError::ExpectedVersionComponent))]
    #[case(
        "*",
        Ok(VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Gte,
                            version: Version {
                                major: 0,
                                minor: 0,
                                patch: 0,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                    ],
                },
            ],
        })
    )]
    #[case(
        "1.x.x",
        Ok(VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![
                        Comparator {
                            op: ComparatorOp::Gte,
                            version: Version {
                                major: 1,
                                minor: 0,
                                patch: 0,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                        Comparator {
                            op: ComparatorOp::Lt,
                            version: Version {
                                major: 2,
                                minor: 0,
                                patch: 0,
                                pre_release: vec![],
                                build: vec![],
                            },
                        },
                    ],
                },
            ],
        })
    )]
    // =============================================================================
    // Wildcard normalization
    // =============================================================================

    // "1", "1.x", "1.*", and "1.X" all represent the same x-range:
    // >=1.0.0 <2.0.0
    #[case(
    "1",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 2,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
    )]
    #[case(
    "1.x",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 2,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
    )]
    #[case(
    "1.*",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 2,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
    )]
    #[case(
    "1.X",
    Ok(VersionRange {
        sets: vec![
            ComparatorSet {
                comparators: vec![
                    Comparator {
                        op: ComparatorOp::Gte,
                        version: Version {
                            major: 1,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                    Comparator {
                        op: ComparatorOp::Lt,
                        version: Version {
                            major: 2,
                            minor: 0,
                            patch: 0,
                            pre_release: vec![],
                            build: vec![],
                        },
                    },
                ],
            },
        ],
    })
)]

    fn test_parser_cases(#[case] input: &str, #[case] expected: Result<VersionRange, ParserError>) {
        let tokens = Lexer::new().parse(input);

        let mut parser = Parser::new(tokens);
        let actual = parser.parse();

        assert!(
            actual == expected,
            "Parser mismatch for input {:?}\n\n{}",
            input,
            format_ast_diff(&actual, &expected),
        );
    }

    // Comprehensive tests for desugaring partial comparators into primitive comparators.
    #[rstest]
    // =============================================================================
    // Caret (^) ranges
    // =============================================================================

    // Exact versions.

    // ^0.0.3 -> >=0.0.3 <0.0.4
    #[case(PartialComparatorOp::Caret, Number(0), Number(0), Number(3), vec![
    (ComparatorOp::Gte, 0, 0, 3),
    (ComparatorOp::Lt, 0, 0, 4),
])]
    // ^0.2.3 -> >=0.2.3 <0.3.0
    #[case(PartialComparatorOp::Caret, Number(0), Number(2), Number(3), vec![
    (ComparatorOp::Gte, 0, 2, 3),
    (ComparatorOp::Lt, 0, 3, 0),
])]
    // ^1.2.3 -> >=1.2.3 <2.0.0
    #[case(PartialComparatorOp::Caret, Number(1), Number(2), Number(3), vec![
    (ComparatorOp::Gte, 1, 2, 3),
    (ComparatorOp::Lt, 2, 0, 0),
])]
    // Partial versions (missing patch).

    // ^0.0 -> >=0.0.0 <0.1.0
    #[case(PartialComparatorOp::Caret, Number(0), Number(0), Wildcard, vec![
    (ComparatorOp::Gte, 0, 0, 0),
    (ComparatorOp::Lt, 0, 1, 0),
])]
    // ^0.2 -> >=0.2.0 <0.3.0
    #[case(PartialComparatorOp::Caret, Number(0), Number(2), Wildcard, vec![
    (ComparatorOp::Gte, 0, 2, 0),
    (ComparatorOp::Lt, 0, 3, 0),
])]
    // ^1.2 -> >=1.2.0 <2.0.0
    #[case(PartialComparatorOp::Caret, Number(1), Number(2), Wildcard, vec![
    (ComparatorOp::Gte, 1, 2, 0),
    (ComparatorOp::Lt, 2, 0, 0),
])]
    // Wildcard major ranges.

    // ^0.x -> >=0.0.0 <1.0.0
    #[case(PartialComparatorOp::Caret, Number(0), Wildcard, Wildcard, vec![
    (ComparatorOp::Gte, 0, 0, 0),
    (ComparatorOp::Lt, 1, 0, 0),
])]
    // ^1.x -> >=1.0.0 <2.0.0
    #[case(PartialComparatorOp::Caret, Number(1), Wildcard, Wildcard, vec![
    (ComparatorOp::Gte, 1, 0, 0),
    (ComparatorOp::Lt, 2, 0, 0),
])]
    // =============================================================================
    // Tilde (~) ranges
    // =============================================================================

    // Exact versions.

    // ~0.2.3 -> >=0.2.3 <0.3.0
    #[case(PartialComparatorOp::Tilde, Number(0), Number(2), Number(3), vec![
    (ComparatorOp::Gte, 0, 2, 3),
    (ComparatorOp::Lt, 0, 3, 0),
])]
    // ~1.2.3 -> >=1.2.3 <1.3.0
    #[case(PartialComparatorOp::Tilde, Number(1), Number(2), Number(3), vec![
    (ComparatorOp::Gte, 1, 2, 3),
    (ComparatorOp::Lt, 1, 3, 0),
])]
    // Partial versions.

    // ~0.2 -> >=0.2.0 <0.3.0
    #[case(PartialComparatorOp::Tilde, Number(0), Number(2), Wildcard, vec![
    (ComparatorOp::Gte, 0, 2, 0),
    (ComparatorOp::Lt, 0, 3, 0),
])]
    // ~1.2 -> >=1.2.0 <1.3.0
    #[case(PartialComparatorOp::Tilde, Number(1), Number(2), Wildcard, vec![
    (ComparatorOp::Gte, 1, 2, 0),
    (ComparatorOp::Lt, 1, 3, 0),
])]
    // Wildcard major ranges.

    // ~0.x -> >=0.0.0 <1.0.0
    #[case(PartialComparatorOp::Tilde, Number(0), Wildcard, Wildcard, vec![
    (ComparatorOp::Gte, 0, 0, 0),
    (ComparatorOp::Lt, 1, 0, 0),
])]
    // ~1.x -> >=1.0.0 <2.0.0
    #[case(PartialComparatorOp::Tilde, Number(1), Wildcard, Wildcard, vec![
    (ComparatorOp::Gte, 1, 0, 0),
    (ComparatorOp::Lt, 2, 0, 0),
])]
    // =============================================================================
    // Exact (=) ranges
    // =============================================================================

    // Fully specified version.

    // =1.2.3 -> =1.2.3
    #[case(PartialComparatorOp::Eq, Number(1), Number(2), Number(3), vec![
    (ComparatorOp::Eq, 1, 2, 3),
])]
    // Partial versions expand to x-ranges.

    // =1.2 -> >=1.2.0 <1.3.0
    #[case(PartialComparatorOp::Eq, Number(1), Number(2), Wildcard, vec![
    (ComparatorOp::Gte, 1, 2, 0),
    (ComparatorOp::Lt, 1, 3, 0),
])]
    // =0.2 -> >=0.2.0 <0.3.0
    #[case(PartialComparatorOp::Eq, Number(0), Number(2), Wildcard, vec![
    (ComparatorOp::Gte, 0, 2, 0),
    (ComparatorOp::Lt, 0, 3, 0),
])]
    // =1 -> >=1.0.0 <2.0.0
    #[case(PartialComparatorOp::Eq, Number(1), Wildcard, Wildcard, vec![
    (ComparatorOp::Gte, 1, 0, 0),
    (ComparatorOp::Lt, 2, 0, 0),
])]
    // =0 -> >=0.0.0 <1.0.0
    #[case(PartialComparatorOp::Eq, Number(0), Wildcard, Wildcard, vec![
    (ComparatorOp::Gte, 0, 0, 0),
    (ComparatorOp::Lt, 1, 0, 0),
])]
    // =============================================================================
    // Comparison operators
    // =============================================================================

    // Fully specified versions remain unchanged.

    // >1.2.3 -> >1.2.3
    #[case(PartialComparatorOp::Gt, Number(1), Number(2), Number(3), vec![
    (ComparatorOp::Gt, 1, 2, 3),
])]
    // >=1.2.3 -> >=1.2.3
    #[case(PartialComparatorOp::Gte, Number(1), Number(2), Number(3), vec![
    (ComparatorOp::Gte, 1, 2, 3),
])]
    // <1.2.3 -> <1.2.3
    #[case(PartialComparatorOp::Lt, Number(1), Number(2), Number(3), vec![
    (ComparatorOp::Lt, 1, 2, 3),
])]
    // <=1.2.3 -> <=1.2.3
    #[case(PartialComparatorOp::Lte, Number(1), Number(2), Number(3), vec![
    (ComparatorOp::Lte, 1, 2, 3),
])]
    // Missing patch is normalized for > and >=.

    // >1.2 -> >1.2.0
    #[case(PartialComparatorOp::Gt, Number(1), Number(2), Wildcard, vec![
    (ComparatorOp::Gt, 1, 2, 0),
])]
    // >=1.2 -> >=1.2.0
    #[case(PartialComparatorOp::Gte, Number(1), Number(2), Wildcard, vec![
    (ComparatorOp::Gte, 1, 2, 0),
])]
    // >1 -> >1.0.0
    #[case(PartialComparatorOp::Gt, Number(1), Wildcard, Wildcard, vec![
    (ComparatorOp::Gt, 1, 0, 0),
])]
    // >=1 -> >=1.0.0
    #[case(PartialComparatorOp::Gte, Number(1), Wildcard, Wildcard, vec![
    (ComparatorOp::Gte, 1, 0, 0),
])]
    // Missing components advance the upper bound for < and <=.

    // <1.2 -> <1.3.0
    #[case(PartialComparatorOp::Lt, Number(1), Number(2), Wildcard, vec![
    (ComparatorOp::Lt, 1, 3, 0),
])]
    // <=1.2 -> <1.3.0
    #[case(PartialComparatorOp::Lte, Number(1), Number(2), Wildcard, vec![
    (ComparatorOp::Lt, 1, 3, 0),
])]
    // <1 -> <2.0.0
    #[case(PartialComparatorOp::Lt, Number(1), Wildcard, Wildcard, vec![
    (ComparatorOp::Lt, 2, 0, 0),
])]
    // <=1 -> <2.0.0
    #[case(PartialComparatorOp::Lte, Number(1), Wildcard, Wildcard, vec![
    (ComparatorOp::Lt, 2, 0, 0),
])]

    fn test_desugar_partial_comparator(
        #[case] op: PartialComparatorOp,
        #[case] major: VersionComponent,
        #[case] minor: VersionComponent,
        #[case] patch: VersionComponent,
        #[case] expected_comps: Vec<(ComparatorOp, u32, u32, u32)>,
    ) {
        let parser = Parser::new(vec![]);
        let input = PartialComparator {
            op,
            version: PartialVersion {
                major,
                minor,
                patch,
                pre_release: vec![],
                build: vec![],
            },
        };

        let expected_result = Ok(expected_comps
            .into_iter()
            .map(|(op, maj, min, pat)| Comparator {
                op,
                version: Version {
                    major: maj,
                    minor: min,
                    patch: pat,
                    pre_release: vec![],
                    build: vec![],
                },
            })
            .collect());

        let actual = parser.desugar_partial_comparator(input);
        assert_eq!(actual, expected_result);
    }
}
