use std::cmp::Ordering;

use crate::parser::{Comparator, ComparatorOp, ComparatorSet, Version, VersionRange};

impl<'a> Ord for Version<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

impl<'a> PartialOrd for Version<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Comparator<'a> {
    pub fn satisfies(&self, version: &Version<'a>) -> bool {
        match self.op {
            ComparatorOp::Eq => version == &self.version,
            ComparatorOp::Gt => version > &self.version,
            ComparatorOp::Gte => version >= &self.version,
            ComparatorOp::Lt => version < &self.version,
            ComparatorOp::Lte => version <= &self.version,
        }
    }
}

impl<'a> ComparatorSet<'a> {
    pub fn satisfies(&self, version: &Version<'a>) -> bool {
        self.comparators
            .iter()
            .all(|comparator| comparator.satisfies(version))
    }
}

impl<'a> VersionRange<'a> {
    pub fn satisfies(&self, version: &Version<'a>) -> bool {
        self.sets.iter().any(|set| set.satisfies(version))
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::{Comparator, ComparatorOp, ComparatorSet, Version, VersionRange};

    fn version(major: u32, minor: u32, patch: u32) -> Version<'static> {
        Version {
            major,
            minor,
            patch,
            pre_release: vec![],
            build: vec![],
        }
    }

    // -------------------------------------------------------------------------
    // Version ordering
    // -------------------------------------------------------------------------

    #[test]
    fn version_eq() {
        assert_eq!(version(1, 2, 3), version(1, 2, 3));
    }

    #[test]
    fn version_lt_major() {
        assert!(version(1, 0, 0) < version(2, 0, 0));
    }

    #[test]
    fn version_lt_minor() {
        assert!(version(1, 2, 0) < version(1, 3, 0));
    }

    #[test]
    fn version_lt_patch() {
        assert!(version(1, 2, 3) < version(1, 2, 4));
    }

    #[test]
    fn version_gt() {
        assert!(version(2, 0, 0) > version(1, 999, 999));
    }

    #[test]
    fn version_lte() {
        assert!(version(1, 2, 3) <= version(1, 2, 3));
    }

    #[test]
    fn version_gte() {
        assert!(version(1, 2, 3) >= version(1, 2, 3));
    }

    // -------------------------------------------------------------------------
    // Comparator
    // -------------------------------------------------------------------------

    #[test]
    fn comparator_eq() {
        let cmp = Comparator {
            op: ComparatorOp::Eq,
            version: version(1, 2, 3),
        };

        assert!(cmp.satisfies(&version(1, 2, 3)));
        assert!(!cmp.satisfies(&version(1, 2, 4)));
    }

    #[test]
    fn comparator_gt() {
        let cmp = Comparator {
            op: ComparatorOp::Gt,
            version: version(1, 2, 3),
        };

        assert!(cmp.satisfies(&version(1, 2, 4)));
        assert!(!cmp.satisfies(&version(1, 2, 3)));
        assert!(!cmp.satisfies(&version(1, 2, 2)));
    }

    #[test]
    fn comparator_gte() {
        let cmp = Comparator {
            op: ComparatorOp::Gte,
            version: version(1, 2, 3),
        };

        assert!(cmp.satisfies(&version(1, 2, 3)));
        assert!(cmp.satisfies(&version(1, 2, 4)));
        assert!(!cmp.satisfies(&version(1, 2, 2)));
    }

    #[test]
    fn comparator_lt() {
        let cmp = Comparator {
            op: ComparatorOp::Lt,
            version: version(1, 2, 3),
        };

        assert!(cmp.satisfies(&version(1, 2, 2)));
        assert!(!cmp.satisfies(&version(1, 2, 3)));
        assert!(!cmp.satisfies(&version(1, 2, 4)));
    }

    #[test]
    fn comparator_lte() {
        let cmp = Comparator {
            op: ComparatorOp::Lte,
            version: version(1, 2, 3),
        };

        assert!(cmp.satisfies(&version(1, 2, 2)));
        assert!(cmp.satisfies(&version(1, 2, 3)));
        assert!(!cmp.satisfies(&version(1, 2, 4)));
    }

    // -------------------------------------------------------------------------
    // ComparatorSet (logical AND)
    // -------------------------------------------------------------------------

    #[test]
    fn comparator_set_satisfied() {
        let set = ComparatorSet {
            comparators: vec![
                Comparator {
                    op: ComparatorOp::Gte,
                    version: version(1, 2, 3),
                },
                Comparator {
                    op: ComparatorOp::Lt,
                    version: version(2, 0, 0),
                },
            ],
        };

        assert!(set.satisfies(&version(1, 5, 0)));
    }

    #[test]
    fn comparator_set_fails_lower_bound() {
        let set = ComparatorSet {
            comparators: vec![
                Comparator {
                    op: ComparatorOp::Gte,
                    version: version(1, 2, 3),
                },
                Comparator {
                    op: ComparatorOp::Lt,
                    version: version(2, 0, 0),
                },
            ],
        };

        assert!(!set.satisfies(&version(1, 2, 2)));
    }

    #[test]
    fn comparator_set_fails_upper_bound() {
        let set = ComparatorSet {
            comparators: vec![
                Comparator {
                    op: ComparatorOp::Gte,
                    version: version(1, 2, 3),
                },
                Comparator {
                    op: ComparatorOp::Lt,
                    version: version(2, 0, 0),
                },
            ],
        };

        assert!(!set.satisfies(&version(2, 0, 0)));
    }

    // -------------------------------------------------------------------------
    // VersionRange (logical OR)
    // -------------------------------------------------------------------------

    #[test]
    fn version_range_first_set_matches() {
        let range = VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![Comparator {
                        op: ComparatorOp::Lt,
                        version: version(2, 0, 0),
                    }],
                },
                ComparatorSet {
                    comparators: vec![Comparator {
                        op: ComparatorOp::Gte,
                        version: version(3, 0, 0),
                    }],
                },
            ],
        };

        assert!(range.satisfies(&version(1, 5, 0)));
    }

    #[test]
    fn version_range_second_set_matches() {
        let range = VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![Comparator {
                        op: ComparatorOp::Lt,
                        version: version(2, 0, 0),
                    }],
                },
                ComparatorSet {
                    comparators: vec![Comparator {
                        op: ComparatorOp::Gte,
                        version: version(3, 0, 0),
                    }],
                },
            ],
        };

        assert!(range.satisfies(&version(3, 1, 0)));
    }

    #[test]
    fn version_range_no_set_matches() {
        let range = VersionRange {
            sets: vec![
                ComparatorSet {
                    comparators: vec![Comparator {
                        op: ComparatorOp::Lt,
                        version: version(2, 0, 0),
                    }],
                },
                ComparatorSet {
                    comparators: vec![Comparator {
                        op: ComparatorOp::Gte,
                        version: version(3, 0, 0),
                    }],
                },
            ],
        };

        assert!(!range.satisfies(&version(2, 5, 0)));
    }
}
