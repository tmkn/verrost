use crate::lexer::Comparator as LexerComparator;
use crate::lexer::Token::{self, Tilde};

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
    AlphaNumeric(&'a str),
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
    major: u32,
    minor: u32,
    patch: u32,

    pre_release: Vec<Identifier<'a>>,
    build: Vec<Identifier<'a>>,
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
pub enum ParserError {
    Generic,
    InvalidComparatorOp,
    ExpectedVersionComponent,
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

    fn tyr_consume_version_range(&mut self) -> bool {
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

        while self.tyr_consume_version_range() {
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
            pre_release: vec![],
            build: vec![],
        })
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
                _ => unreachable!(),
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
                _ => panic!(),
            },

            _ => Err(ParserError::Generic),
        }
    }
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
