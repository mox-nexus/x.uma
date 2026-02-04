//! Conformance test fixture runner
//!
//! Loads YAML fixtures and runs them against the rumi engine.

use rumi::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

/// A complete test fixture
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub name: String,
    pub description: String,
    pub matcher: MatcherConfig,
    pub cases: Vec<TestCase>,
}

/// Matcher configuration from YAML
#[derive(Debug, Deserialize)]
pub struct MatcherConfig {
    pub matchers: Vec<FieldMatcherConfig>,
    #[serde(default)]
    pub on_no_match: Option<Box<OnMatchConfig>>,
}

/// Field matcher configuration
#[derive(Debug, Deserialize)]
pub struct FieldMatcherConfig {
    pub predicate: PredicateConfig,
    pub on_match: OnMatchConfig,
}

/// Predicate configuration (single, and, or, not)
/// Uses untagged deserialization - order matters!
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PredicateConfig {
    // Try composite predicates first (they have specific keys)
    And(AndPredicate),
    Or(OrPredicate),
    Not(NotPredicate),
    // Single last (most general structure)
    Single(SinglePredicate_),
}

#[derive(Debug, Deserialize)]
pub struct SinglePredicate_ {
    pub single: SinglePredicateConfig,
}

#[derive(Debug, Deserialize)]
pub struct AndPredicate {
    pub and: Vec<PredicateConfig>,
}

#[derive(Debug, Deserialize)]
pub struct OrPredicate {
    pub or: Vec<PredicateConfig>,
}

#[derive(Debug, Deserialize)]
pub struct NotPredicate {
    pub not: Box<PredicateConfig>,
}

/// Single predicate: input + value_match
#[derive(Debug, Deserialize)]
pub struct SinglePredicateConfig {
    pub input: InputConfig,
    pub value_match: ValueMatchConfig,
}

/// Input configuration (just a key for TestContext)
#[derive(Debug, Deserialize)]
pub struct InputConfig {
    pub key: String,
}

/// Value match configuration
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ValueMatchConfig {
    Exact(ExactMatch),
    Prefix(PrefixMatch),
    Suffix(SuffixMatch),
    Contains(ContainsMatch),
}

#[derive(Debug, Deserialize)]
pub struct ExactMatch {
    pub exact: String,
}

#[derive(Debug, Deserialize)]
pub struct PrefixMatch {
    pub prefix: String,
}

#[derive(Debug, Deserialize)]
pub struct SuffixMatch {
    pub suffix: String,
}

#[derive(Debug, Deserialize)]
pub struct ContainsMatch {
    pub contains: String,
}

/// OnMatch configuration: action or nested matcher
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum OnMatchConfig {
    Matcher(MatcherOnMatch),
    Action(ActionOnMatch),
}

#[derive(Debug, Deserialize)]
pub struct ActionOnMatch {
    pub action: String,
}

#[derive(Debug, Deserialize)]
pub struct MatcherOnMatch {
    pub matcher: Box<MatcherConfig>,
}

/// Test case
#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub context: HashMap<String, String>,
    pub expect: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// Builder: Convert config to rumi types
// ═══════════════════════════════════════════════════════════════════════════════

use crate::{StringInput, TestContext};

impl MatcherConfig {
    /// Build a rumi Matcher from this config
    pub fn build(&self) -> Matcher<TestContext, String> {
        let field_matchers = self
            .matchers
            .iter()
            .map(FieldMatcherConfig::build)
            .collect();
        let on_no_match = self.on_no_match.as_ref().map(|om| om.build());
        Matcher::new(field_matchers, on_no_match)
    }
}

impl FieldMatcherConfig {
    fn build(&self) -> FieldMatcher<TestContext, String> {
        FieldMatcher::new(self.predicate.build(), self.on_match.build())
    }
}

impl PredicateConfig {
    fn build(&self) -> Predicate<TestContext> {
        match self {
            PredicateConfig::Single(s) => Predicate::Single(s.single.build()),
            PredicateConfig::And(a) => {
                Predicate::And(a.and.iter().map(PredicateConfig::build).collect())
            }
            PredicateConfig::Or(o) => {
                Predicate::Or(o.or.iter().map(PredicateConfig::build).collect())
            }
            PredicateConfig::Not(n) => Predicate::Not(Box::new(n.not.build())),
        }
    }
}

impl SinglePredicateConfig {
    fn build(&self) -> SinglePredicate<TestContext> {
        let input: Box<dyn DataInput<TestContext>> = Box::new(StringInput::new(&self.input.key));
        let matcher: Box<dyn InputMatcher> = self.value_match.build();
        SinglePredicate::new(input, matcher)
    }
}

impl ValueMatchConfig {
    fn build(&self) -> Box<dyn InputMatcher> {
        match self {
            ValueMatchConfig::Exact(e) => Box::new(ExactMatcher::new(&e.exact)),
            ValueMatchConfig::Prefix(p) => Box::new(PrefixMatcher::new(&p.prefix)),
            ValueMatchConfig::Suffix(s) => Box::new(SuffixMatcher::new(&s.suffix)),
            ValueMatchConfig::Contains(c) => Box::new(ContainsMatcher::new(&c.contains)),
        }
    }
}

impl OnMatchConfig {
    fn build(&self) -> OnMatch<TestContext, String> {
        match self {
            OnMatchConfig::Action(a) => OnMatch::Action(a.action.clone()),
            OnMatchConfig::Matcher(m) => OnMatch::Matcher(Box::new(m.matcher.build())),
        }
    }
}

impl TestCase {
    /// Build a TestContext from this case's context map
    pub fn build_context(&self) -> TestContext {
        let mut ctx = TestContext::new();
        for (k, v) in &self.context {
            ctx = ctx.with(k.clone(), v.clone());
        }
        ctx
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Runner
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of running a single test case
#[derive(Debug)]
pub struct CaseResult {
    pub case_name: String,
    pub passed: bool,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl Fixture {
    /// Parse a fixture from YAML
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Parse multiple fixtures from a YAML file with `---` separators
    pub fn from_yaml_multi(yaml: &str) -> Result<Vec<Self>, serde_yaml::Error> {
        let mut fixtures = Vec::new();
        for doc in serde_yaml::Deserializer::from_str(yaml) {
            fixtures.push(Self::deserialize(doc)?);
        }
        Ok(fixtures)
    }

    /// Run all test cases and return results
    pub fn run(&self) -> Vec<CaseResult> {
        let matcher = self.matcher.build();
        self.cases
            .iter()
            .map(|case| {
                let ctx = case.build_context();
                let actual = matcher.evaluate(&ctx);
                CaseResult {
                    case_name: case.name.clone(),
                    passed: actual == case.expect,
                    expected: case.expect.clone(),
                    actual,
                }
            })
            .collect()
    }

    /// Run all test cases and panic on first failure
    pub fn run_and_assert(&self) {
        let results = self.run();
        for result in results {
            assert!(
                result.passed,
                "Fixture '{}' case '{}' failed: expected {:?}, got {:?}",
                self.name, result.case_name, result.expected, result.actual
            );
        }
    }
}
