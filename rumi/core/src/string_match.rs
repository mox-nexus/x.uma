//! `StringMatchSpec` — Config-level string match specification
//!
//! This type represents a user's *intent* for string matching (e.g., "exact match on /api").
//! It compiles to runtime [`InputMatcher`] types via [`to_input_matcher()`](StringMatchSpec::to_input_matcher).
//!
//! # Naming: Spec vs Matcher
//!
//! - [`StringMatchSpec`] = config-level specification (what the user wrote)
//! - [`StringMatcher`](crate::StringMatcher) = runtime engine (what evaluates at match time)
//!
//! The `Spec` suffix makes the ontological distinction clear (Karman: guild review).

use crate::{DataInput, InputMatcher, MatcherError, Predicate, SinglePredicate, StringMatcher};
use std::fmt;

/// A string match specification from user configuration.
///
/// Represents one of five matching strategies. Compiles to the appropriate
/// runtime [`InputMatcher`] via [`to_input_matcher()`](Self::to_input_matcher).
///
/// # Example
///
/// ```
/// use rumi::StringMatchSpec;
///
/// let spec = StringMatchSpec::Prefix("/api".into());
/// let matcher = spec.to_input_matcher().unwrap();
///
/// // Or compile directly to a Predicate with a DataInput:
/// // let predicate = spec.to_predicate(Box::new(PathInput))?;
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StringMatchSpec {
    /// Exact string equality.
    Exact(String),
    /// String starts with prefix.
    Prefix(String),
    /// String ends with suffix.
    Suffix(String),
    /// String contains substring.
    Contains(String),
    /// Regular expression match (Rust `regex` crate syntax, linear time).
    Regex(String),
}

impl StringMatchSpec {
    /// Compile this spec into a runtime [`InputMatcher`].
    ///
    /// Pattern length limits are enforced **here**, in the constructor, not in
    /// the config loader. This is the only path from a spec to a matcher, so
    /// every caller inherits the guarantee — the registry, both domain
    /// compilers, and anyone constructing a spec by hand.
    ///
    /// They lived in a private `Registry` method until 2026-08-17, which meant
    /// `HookMatch::compile` accepted an 8 MB pattern against an 8192-byte limit.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError::PatternTooLong`] if the pattern exceeds
    /// [`MAX_PATTERN_LENGTH`](crate::MAX_PATTERN_LENGTH), or
    /// [`MAX_REGEX_PATTERN_LENGTH`](crate::MAX_REGEX_PATTERN_LENGTH) for a
    /// regex. Returns [`MatcherError::InvalidPattern`] if the regex is invalid.
    pub fn to_input_matcher(&self) -> Result<Box<dyn InputMatcher>, MatcherError> {
        self.to_input_matcher_with_case(false)
    }

    /// Compile this spec, optionally matching case-insensitively.
    ///
    /// This is xDS `StringMatcher.ignore_case`. The flag was read from the
    /// proto and thrown away until 2026-08-18, under a comment asserting the
    /// registry handled it — a rule that read case-insensitive and was not.
    ///
    /// # Errors
    ///
    /// As [`to_input_matcher`](Self::to_input_matcher), plus
    /// [`MatcherError::InvalidPattern`] when `ignore_case` is set on a regex
    /// that countermands it inline — see [`disables_case_insensitivity`].
    pub fn to_input_matcher_with_case(
        &self,
        ignore_case: bool,
    ) -> Result<Box<dyn InputMatcher>, MatcherError> {
        self.check_length()?;

        let boxed = |sm: StringMatcher| Box::new(sm) as Box<dyn InputMatcher>;
        match self {
            Self::Exact(v) => Ok(boxed(StringMatcher::exact(v.clone(), ignore_case))),
            Self::Prefix(v) => Ok(boxed(StringMatcher::prefix(v.clone(), ignore_case))),
            Self::Suffix(v) => Ok(boxed(StringMatcher::suffix(v.clone(), ignore_case))),
            Self::Contains(v) => Ok(boxed(StringMatcher::contains(v.clone(), ignore_case))),
            Self::Regex(v) => {
                if ignore_case && disables_case_insensitivity(v) {
                    return Err(MatcherError::InvalidPattern {
                        pattern: v.clone(),
                        source: "ignore_case is set, but the pattern turns case-insensitivity \
                                 off inline with a (?-i) flag. An inline flag wins, so this \
                                 rule would read case-insensitive and not be. Remove one of \
                                 the two."
                            .into(),
                    });
                }
                let compiled = if ignore_case {
                    StringMatcher::regex_ignore_case(v)
                } else {
                    StringMatcher::regex(v)
                };
                compiled
                    .map(boxed)
                    .map_err(|e| MatcherError::InvalidPattern {
                        pattern: v.clone(),
                        source: e.to_string(),
                    })
            }
        }
    }

    /// Does this pattern turn case-insensitivity off with an inline flag?
    ///
    /// `ignore_case` is implemented by asking the regex engine for a
    /// case-insensitive match, and an inline `(?-i)` overrides that — measured
    /// both ways on 2026-08-18: `(?i)(?-i)admin` and
    /// `RegexBuilder::case_insensitive(true)` on `(?-i)admin` both fail to
    /// match `ADMIN`. That is correct regex semantics and not something a
    /// different construction can fix, so the combination is rejected instead.
    ///
    /// Scans flag groups — `(?` followed by flag letters, then `)` or `:` — and
    /// reports any that clear `i`. A group with no `-` cannot clear anything,
    /// and `(?i-s)` clears `s`, not `i`.
    /// Enforce the pattern length limit for this variant.
    ///
    /// Regexes get the tighter [`MAX_REGEX_PATTERN_LENGTH`](crate::MAX_REGEX_PATTERN_LENGTH)
    /// because compiled program size, not pattern length, drives their cost.
    fn check_length(&self) -> Result<(), MatcherError> {
        let (len, max) = match self {
            Self::Regex(v) => (v.len(), crate::MAX_REGEX_PATTERN_LENGTH),
            Self::Exact(v) | Self::Prefix(v) | Self::Suffix(v) | Self::Contains(v) => {
                (v.len(), crate::MAX_PATTERN_LENGTH)
            }
        };
        if len > max {
            return Err(MatcherError::PatternTooLong { len, max });
        }
        Ok(())
    }

    /// Compile this spec into a [`Predicate`] with the given [`DataInput`].
    ///
    /// Equivalent to `Predicate::Single(SinglePredicate::new(input, self.to_input_matcher()?))`.
    ///
    /// # Errors
    ///
    /// Returns [`MatcherError::InvalidPattern`] if the regex is invalid.
    pub fn to_predicate<Ctx: 'static>(
        &self,
        input: Box<dyn DataInput<Ctx>>,
    ) -> Result<Predicate<Ctx>, MatcherError> {
        let matcher = self.to_input_matcher()?;
        Ok(Predicate::Single(SinglePredicate::new(input, matcher)))
    }
}

impl fmt::Display for StringMatchSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "Exact(\"{v}\")"),
            Self::Prefix(v) => write!(f, "Prefix(\"{v}\")"),
            Self::Suffix(v) => write!(f, "Suffix(\"{v}\")"),
            Self::Contains(v) => write!(f, "Contains(\"{v}\")"),
            Self::Regex(v) => write!(f, "Regex(\"{v}\")"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MatchingData;

    #[test]
    fn exact_compiles() {
        let spec = StringMatchSpec::Exact("hello".into());
        let m = spec.to_input_matcher().unwrap();
        assert!(m.matches(&MatchingData::String("hello".into())));
        assert!(!m.matches(&MatchingData::String("world".into())));
    }

    #[test]
    fn prefix_compiles() {
        let spec = StringMatchSpec::Prefix("/api".into());
        let m = spec.to_input_matcher().unwrap();
        assert!(m.matches(&MatchingData::String("/api/users".into())));
        assert!(!m.matches(&MatchingData::String("/other".into())));
    }

    #[test]
    fn suffix_compiles() {
        let spec = StringMatchSpec::Suffix(".rs".into());
        let m = spec.to_input_matcher().unwrap();
        assert!(m.matches(&MatchingData::String("main.rs".into())));
        assert!(!m.matches(&MatchingData::String("main.py".into())));
    }

    #[test]
    fn contains_compiles() {
        let spec = StringMatchSpec::Contains("error".into());
        let m = spec.to_input_matcher().unwrap();
        assert!(m.matches(&MatchingData::String("an error occurred".into())));
        assert!(!m.matches(&MatchingData::String("success".into())));
    }

    #[test]
    fn regex_compiles() {
        let spec = StringMatchSpec::Regex(r"^user-\d+$".into());
        let m = spec.to_input_matcher().unwrap();
        assert!(m.matches(&MatchingData::String("user-123".into())));
        assert!(!m.matches(&MatchingData::String("user-abc".into())));
    }

    #[test]
    fn invalid_regex_returns_error() {
        let spec = StringMatchSpec::Regex("[bad".into());
        let err = spec.to_input_matcher().unwrap_err();
        assert!(matches!(err, MatcherError::InvalidPattern { .. }));
    }

    #[test]
    fn to_predicate_compiles() {
        #[derive(Debug)]
        struct Ctx {
            val: String,
        }
        #[derive(Debug)]
        struct ValInput;
        impl DataInput<Ctx> for ValInput {
            fn get(&self, ctx: &Ctx) -> MatchingData {
                MatchingData::String(ctx.val.clone())
            }
        }

        let spec = StringMatchSpec::Exact("hello".into());
        let pred = spec.to_predicate(Box::new(ValInput)).unwrap();

        let ctx = Ctx {
            val: "hello".into(),
        };
        assert!(pred.evaluate(&ctx));

        let ctx = Ctx {
            val: "world".into(),
        };
        assert!(!pred.evaluate(&ctx));
    }

    #[test]
    fn display() {
        assert_eq!(
            StringMatchSpec::Exact("Bash".into()).to_string(),
            r#"Exact("Bash")"#
        );
        assert_eq!(
            StringMatchSpec::Regex("^mcp".into()).to_string(),
            r#"Regex("^mcp")"#
        );
    }

    // ── Regression: SEC2 / review F-02 ──────────────────────────────────────
    //
    // Limits used to live in a private Registry method, so every path that did
    // not go through the JSON/YAML loader was unprotected: both domain
    // compilers, and anyone building a spec by hand. The review demonstrated
    // HookMatch::compile accepting an 8 MB pattern against an 8192-byte limit.
    //
    // These assert the constructor itself refuses. If someone moves the check
    // back out to a caller, these fail.

    #[test]
    fn oversized_literal_is_rejected_by_the_constructor() {
        let huge = "A".repeat(crate::MAX_PATTERN_LENGTH + 1);
        for spec in [
            StringMatchSpec::Exact(huge.clone()),
            StringMatchSpec::Prefix(huge.clone()),
            StringMatchSpec::Suffix(huge.clone()),
            StringMatchSpec::Contains(huge),
        ] {
            let err = spec
                .to_input_matcher()
                .expect_err("must reject oversized pattern");
            assert!(
                matches!(err, MatcherError::PatternTooLong { .. }),
                "expected PatternTooLong for {spec}, got {err:?}"
            );
        }
    }

    #[test]
    fn oversized_regex_is_rejected_by_the_constructor() {
        let huge = "a".repeat(crate::MAX_REGEX_PATTERN_LENGTH + 1);
        let err = StringMatchSpec::Regex(huge)
            .to_input_matcher()
            .expect_err("must reject oversized regex");
        assert!(
            matches!(err, MatcherError::PatternTooLong { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn regex_limit_is_tighter_than_literal_limit() {
        // A pattern between the two limits: fine as a literal, refused as a
        // regex. Proves the two are distinct rather than one check reused, and
        // fails if someone unifies them.
        let mid = "a".repeat(crate::MAX_REGEX_PATTERN_LENGTH + 1);
        assert!(
            mid.len() <= crate::MAX_PATTERN_LENGTH,
            "test premise: limits differ"
        );
        assert!(StringMatchSpec::Exact(mid.clone())
            .to_input_matcher()
            .is_ok());
        assert!(StringMatchSpec::Regex(mid).to_input_matcher().is_err());
    }

    #[test]
    fn the_guard_is_not_inert() {
        // A limit test that only ever asserts rejection passes just as well
        // against a constructor that rejects everything. This is the other half.
        let ok = "A".repeat(crate::MAX_PATTERN_LENGTH);
        assert!(StringMatchSpec::Exact(ok).to_input_matcher().is_ok());
        let ok_re = "a".repeat(crate::MAX_REGEX_PATTERN_LENGTH);
        assert!(StringMatchSpec::Regex(ok_re).to_input_matcher().is_ok());
    }
}

/// Does this pattern clear the `i` flag with an inline group?
///
/// `ignore_case` asks the regex engine for a case-insensitive match, and an
/// inline `(?-i)` overrides that. Measured both ways on 2026-08-18:
/// `(?i)(?-i)admin` and `RegexBuilder::case_insensitive(true)` applied to
/// `(?-i)admin` both fail to match `ADMIN`. That is correct regex semantics, so
/// no choice of construction fixes it — the combination has to be rejected.
///
/// Scans flag groups: `(?` followed by flag letters, terminated by `)` or `:`.
/// A group clears `i` only if an `i` appears after a `-` within it, so `(?i-s)`
/// is fine and `(?-si)` is not. Escaped `\(` is not a group.
fn disables_case_insensitivity(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    let mut i = 0;

    while i + 1 < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] != b'(' || bytes[i + 1] != b'?' {
            i += 1;
            continue;
        }

        // Walk the flag letters. `-` switches from setting to clearing.
        let mut j = i + 2;
        let mut clearing = false;
        while j < bytes.len() {
            match bytes[j] {
                b'-' => clearing = true,
                b'i' if clearing => return true,
                b'i' | b'm' | b's' | b'u' | b'x' | b'U' => {}
                // `)` and `:` end a flag group; anything else means this was
                // some other `(?` construct — a named group, a lookaround.
                _ => break,
            }
            j += 1;
        }
        i = j.max(i + 2);
    }

    false
}

#[cfg(test)]
mod ignore_case_tests {
    use super::*;
    use crate::MatchingData;

    fn matches(spec: &StringMatchSpec, ignore_case: bool, input: &str) -> bool {
        spec.to_input_matcher_with_case(ignore_case)
            .unwrap()
            .matches(&MatchingData::String(input.into()))
    }

    /// The flag was read from the proto and discarded, so a rule that read
    /// case-insensitive matched case-sensitively.
    #[test]
    fn ignore_case_reaches_every_literal_variant() {
        for spec in [
            StringMatchSpec::Exact("admin".into()),
            StringMatchSpec::Prefix("admin".into()),
            StringMatchSpec::Suffix("admin".into()),
            StringMatchSpec::Contains("admin".into()),
        ] {
            assert!(matches(&spec, true, "ADMIN"), "{spec:?} with ignore_case");
            assert!(!matches(&spec, false, "ADMIN"), "{spec:?} without");
        }
    }

    #[test]
    fn ignore_case_reaches_regex() {
        let spec = StringMatchSpec::Regex("^admin$".into());
        assert!(matches(&spec, true, "ADMIN"));
        assert!(!matches(&spec, false, "ADMIN"));
    }

    /// An inline `(?-i)` beats the engine's case-insensitive setting, whichever
    /// way that setting is applied. So the pair is refused rather than silently
    /// producing a rule that reads one way and behaves another.
    #[test]
    fn ignore_case_with_an_inline_off_switch_is_rejected() {
        for pattern in ["(?-i)admin", "a(?-i)dmin", "(?-si)admin", "(?s-i)admin"] {
            let spec = StringMatchSpec::Regex(pattern.into());
            let err = spec.to_input_matcher_with_case(true).unwrap_err();
            assert!(
                matches!(err, MatcherError::InvalidPattern { .. }),
                "{pattern} should be rejected, got {err:?}"
            );
            // Without ignore_case there is nothing to contradict.
            assert!(spec.to_input_matcher_with_case(false).is_ok(), "{pattern}");
        }
    }

    /// The guard must not reject patterns that clear some *other* flag, or that
    /// merely contain the characters somewhere harmless.
    #[test]
    fn the_inline_flag_guard_does_not_overreach() {
        for pattern in [
            "(?i-s)admin",     // clears s, not i
            "(?s)admin",       // sets s
            r"\(?-i\)admin",   // escaped, not a group
            "admin-i",         // no group at all
            "(?:admin)",       // non-capturing group
            "(?P<name>admin)", // named group
        ] {
            let spec = StringMatchSpec::Regex(pattern.into());
            assert!(
                spec.to_input_matcher_with_case(true).is_ok(),
                "{pattern} should be accepted"
            );
        }
    }
}
