//! Compiler: `HookMatch` -> rumi `Matcher<HookContext, A>`
//!
//! Translates user-friendly hook match configuration into efficient
//! runtime matchers operating on [`HookContext`].

use crate::config::{ArgumentMatch, CompileError, HookMatch, StringMatch};
use crate::context::{HookContext, HookEvent};
use crate::inputs::{ArgumentInput, CwdInput, EventInput, GitBranchInput, ToolNameInput};
use rumi::prelude::*;

/// Extension trait for compiling `HookMatch` to rumi `Matcher`.
pub trait HookMatchExt {
    /// Compile this `HookMatch` into a rumi `Matcher`.
    ///
    /// The resulting matcher operates on [`HookContext`] and returns
    /// the provided action when all conditions match.
    ///
    /// # Errors
    ///
    /// Returns [`CompileError`] if any regex pattern is invalid.
    fn compile<A: Clone + Send + Sync + 'static>(
        &self,
        action: A,
    ) -> Result<Matcher<HookContext, A>, CompileError>;

    /// Compile this `HookMatch` into a `Predicate` (without action).
    ///
    /// # Errors
    ///
    /// Returns [`CompileError`] if any regex pattern is invalid.
    fn to_predicate(&self) -> Result<Predicate<HookContext>, CompileError>;
}

impl HookMatchExt for HookMatch {
    fn compile<A: Clone + Send + Sync + 'static>(
        &self,
        action: A,
    ) -> Result<Matcher<HookContext, A>, CompileError> {
        let predicate = self.to_predicate()?;
        Ok(Matcher::new(
            vec![FieldMatcher::new(predicate, OnMatch::Action(action))],
            None,
        ))
    }

    fn to_predicate(&self) -> Result<Predicate<HookContext>, CompileError> {
        let mut predicates: Vec<Predicate<HookContext>> = Vec::new();

        // Event matching (exact only — event names are a known enum)
        if let Some(event) = &self.event {
            predicates.push(compile_event_match(*event));
        }

        // Tool name matching
        if let Some(tool_match) = &self.tool_name {
            predicates.push(compile_string_predicate(
                Box::new(ToolNameInput),
                tool_match,
            )?);
        }

        // Argument matching (all ANDed)
        if let Some(arguments) = &self.arguments {
            for arg_match in arguments {
                predicates.push(compile_argument_match(arg_match)?);
            }
        }

        // CWD matching
        if let Some(cwd_match) = &self.cwd {
            predicates.push(compile_string_predicate(Box::new(CwdInput), cwd_match)?);
        }

        // Git branch matching
        if let Some(branch_match) = &self.git_branch {
            predicates.push(compile_string_predicate(
                Box::new(GitBranchInput),
                branch_match,
            )?);
        }

        // Empty match = match everything (same as rumi-http)
        if predicates.is_empty() {
            return Ok(Predicate::Single(SinglePredicate::new(
                Box::new(EventInput),
                Box::new(PrefixMatcher::new("")), // matches any event string
            )));
        }

        if predicates.len() == 1 {
            Ok(predicates.pop().unwrap())
        } else {
            Ok(Predicate::And(predicates))
        }
    }
}

/// Compile an event match to a predicate (always exact).
fn compile_event_match(event: HookEvent) -> Predicate<HookContext> {
    Predicate::Single(SinglePredicate::new(
        Box::new(EventInput),
        Box::new(ExactMatcher::new(event.as_str())),
    ))
}

/// Compile a `StringMatch` + `DataInput` into a predicate.
fn compile_string_predicate(
    input: Box<dyn DataInput<HookContext>>,
    string_match: &StringMatch,
) -> Result<Predicate<HookContext>, CompileError> {
    let matcher = string_match.to_input_matcher()?;
    Ok(Predicate::Single(SinglePredicate::new(input, matcher)))
}

/// Compile an argument match to a predicate.
fn compile_argument_match(
    arg_match: &ArgumentMatch,
) -> Result<Predicate<HookContext>, CompileError> {
    let input = Box::new(ArgumentInput::new(&arg_match.name));
    let matcher = arg_match.value.to_input_matcher()?;
    Ok(Predicate::Single(SinglePredicate::new(input, matcher)))
}

/// Compile multiple `HookMatch` entries into a single `Matcher`.
///
/// Multiple matches are `ORed` together: the first matching rule wins.
///
/// # Errors
///
/// Returns [`CompileError`] if any regex pattern is invalid.
pub fn compile_hook_matches<A: Clone + Send + Sync + 'static>(
    matches: &[HookMatch],
    action: A,
    on_no_match: Option<A>,
) -> Result<Matcher<HookContext, A>, CompileError> {
    if matches.is_empty() {
        // Empty matches = match everything
        return Ok(Matcher::new(
            vec![FieldMatcher::new(
                Predicate::Single(SinglePredicate::new(
                    Box::new(EventInput),
                    Box::new(PrefixMatcher::new("")),
                )),
                OnMatch::Action(action),
            )],
            on_no_match.map(OnMatch::Action),
        ));
    }

    if matches.len() == 1 {
        let predicate = matches[0].to_predicate()?;
        return Ok(Matcher::new(
            vec![FieldMatcher::new(predicate, OnMatch::Action(action))],
            on_no_match.map(OnMatch::Action),
        ));
    }

    // Multiple matches: OR them together
    let predicates: Vec<Predicate<HookContext>> = matches
        .iter()
        .map(HookMatchExt::to_predicate)
        .collect::<Result<_, _>>()?;

    Ok(Matcher::new(
        vec![FieldMatcher::new(
            Predicate::Or(predicates),
            OnMatch::Action(action),
        )],
        on_no_match.map(OnMatch::Action),
    ))
}

// ═══════════════════════════════════════════════════════════════════════════════
// Trace: debug "why did this match / not match?"
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of tracing a `HookMatch` against a `HookContext`.
///
/// Shows which fields matched and which didn't, with expected vs actual values.
#[derive(Debug)]
pub struct HookMatchTrace {
    /// Whether the overall match succeeded (all steps must match).
    pub matched: bool,
    /// Per-field trace steps.
    pub steps: Vec<TraceStep>,
}

/// One field's evaluation result in a trace.
#[derive(Debug)]
pub struct TraceStep {
    /// Field name (e.g., "event", "`tool_name`", "argument[command]").
    pub field: String,
    /// What was expected (e.g., `Exact("Bash")`).
    pub expected: String,
    /// What was found in the context.
    pub actual: String,
    /// Whether this individual step matched.
    pub matched: bool,
}

impl HookMatch {
    /// Trace this match against a context, reporting per-field results.
    ///
    /// Use this to debug "why was this hook triggered?" or "why didn't it match?"
    #[must_use]
    pub fn trace(&self, ctx: &HookContext) -> HookMatchTrace {
        let mut steps = Vec::new();

        if let Some(event) = &self.event {
            let actual = ctx.event().as_str().to_string();
            let expected_str = event.as_str().to_string();
            let matched = actual == expected_str;
            steps.push(TraceStep {
                field: "event".into(),
                expected: expected_str,
                actual,
                matched,
            });
        }

        if let Some(tool_match) = &self.tool_name {
            let actual = ctx.tool_name().to_string();
            let matched = trace_string_match(tool_match, &actual);
            steps.push(TraceStep {
                field: "tool_name".into(),
                expected: tool_match.to_string(),
                actual,
                matched,
            });
        }

        if let Some(arguments) = &self.arguments {
            for arg_match in arguments {
                let actual = ctx
                    .argument(&arg_match.name)
                    .unwrap_or("<missing>")
                    .to_string();
                let matched = ctx
                    .argument(&arg_match.name)
                    .is_some_and(|v| trace_string_match(&arg_match.value, v));
                steps.push(TraceStep {
                    field: format!("argument[{}]", arg_match.name),
                    expected: arg_match.value.to_string(),
                    actual,
                    matched,
                });
            }
        }

        if let Some(cwd_match) = &self.cwd {
            let actual = ctx.cwd().to_string();
            let matched = trace_string_match(cwd_match, &actual);
            steps.push(TraceStep {
                field: "cwd".into(),
                expected: cwd_match.to_string(),
                actual,
                matched,
            });
        }

        if let Some(branch_match) = &self.git_branch {
            let actual = ctx.git_branch().unwrap_or("<none>").to_string();
            let matched = ctx
                .git_branch()
                .is_some_and(|v| trace_string_match(branch_match, v));
            steps.push(TraceStep {
                field: "git_branch".into(),
                expected: branch_match.to_string(),
                actual,
                matched,
            });
        }

        let overall = steps.iter().all(|s| s.matched);
        HookMatchTrace {
            matched: overall,
            steps,
        }
    }
}

/// Evaluate a `StringMatch` against a concrete value (for trace only).
///
/// Compiles the `StringMatch` to an `InputMatcher` and checks against
/// `MatchingData::String`. Returns false if the regex is invalid.
fn trace_string_match(string_match: &StringMatch, value: &str) -> bool {
    string_match
        .to_input_matcher()
        .map(|m| m.matches(&MatchingData::String(value.to_string())))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Predicate Structure Tests ==========

    #[test]
    fn empty_hook_match_compiles_to_single_predicate() {
        let m = HookMatch::default();
        let predicate = m.to_predicate().unwrap();
        assert!(matches!(predicate, Predicate::Single(_)));
    }

    #[test]
    fn single_field_compiles_to_single_predicate() {
        let m = HookMatch {
            event: Some(HookEvent::PreToolUse),
            ..Default::default()
        };
        let predicate = m.to_predicate().unwrap();
        assert!(matches!(predicate, Predicate::Single(_)));
    }

    #[test]
    fn multiple_fields_compiles_to_and_predicate() {
        let m = HookMatch {
            event: Some(HookEvent::PreToolUse),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        };
        let predicate = m.to_predicate().unwrap();
        assert!(matches!(predicate, Predicate::And(_)));
    }

    // ========== E2E: Event Matching ==========

    #[test]
    fn e2e_event_match() {
        let m = HookMatch {
            event: Some(HookEvent::PreToolUse),
            ..Default::default()
        };
        let matcher = m.compile("matched").unwrap();

        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), Some("matched"));

        let ctx = HookContext::post_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), None);

        let ctx = HookContext::stop();
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn e2e_event_match_session_start() {
        let m = HookMatch {
            event: Some(HookEvent::SessionStart),
            ..Default::default()
        };
        let matcher = m.compile("init").unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::session_start()),
            Some("init")
        );
        assert_eq!(matcher.evaluate(&HookContext::session_end()), None);
        assert_eq!(matcher.evaluate(&HookContext::pre_tool_use("X")), None);
    }

    // ========== E2E: Tool Name Matching ==========

    #[test]
    fn e2e_tool_exact_match() {
        let m = HookMatch {
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        };
        let matcher = m.compile("found").unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Bash")),
            Some("found")
        );
        assert_eq!(matcher.evaluate(&HookContext::pre_tool_use("Write")), None);
    }

    #[test]
    fn e2e_tool_prefix_match() {
        let m = HookMatch {
            tool_name: Some(StringMatch::Prefix("mcp__".into())),
            ..Default::default()
        };
        let matcher = m.compile("mcp_tool").unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("mcp__db__query")),
            Some("mcp_tool")
        );
        assert_eq!(matcher.evaluate(&HookContext::pre_tool_use("Bash")), None);
    }

    #[test]
    fn e2e_tool_regex_match() {
        let m = HookMatch {
            tool_name: Some(StringMatch::Regex(r"^(Write|Edit)$".into())),
            ..Default::default()
        };
        let matcher = m.compile("file_op").unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Write")),
            Some("file_op")
        );
        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Edit")),
            Some("file_op")
        );
        assert_eq!(matcher.evaluate(&HookContext::pre_tool_use("Read")), None);
    }

    // ========== E2E: Argument Matching ==========

    #[test]
    fn e2e_argument_contains_match() {
        let m = HookMatch {
            arguments: Some(vec![ArgumentMatch {
                name: "command".into(),
                value: StringMatch::Contains("rm -rf".into()),
            }]),
            ..Default::default()
        };
        let matcher = m.compile("dangerous").unwrap();

        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "sudo rm -rf /");
        assert_eq!(matcher.evaluate(&ctx), Some("dangerous"));

        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "ls -la");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn e2e_argument_missing_returns_no_match() {
        let m = HookMatch {
            arguments: Some(vec![ArgumentMatch {
                name: "command".into(),
                value: StringMatch::Exact("ls".into()),
            }]),
            ..Default::default()
        };
        let matcher = m.compile("found").unwrap();

        // No arguments at all — DataInput returns None → predicate false
        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    // ========== E2E: CWD + Git Branch ==========

    #[test]
    fn e2e_cwd_prefix_match() {
        let m = HookMatch {
            cwd: Some(StringMatch::Prefix("/home/user/project".into())),
            ..Default::default()
        };
        let matcher = m.compile("in_project").unwrap();

        let ctx = HookContext::pre_tool_use("Bash").with_cwd("/home/user/project/src");
        assert_eq!(matcher.evaluate(&ctx), Some("in_project"));

        let ctx = HookContext::pre_tool_use("Bash").with_cwd("/tmp");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn e2e_git_branch_match() {
        let m = HookMatch {
            git_branch: Some(StringMatch::Exact("main".into())),
            ..Default::default()
        };
        let matcher = m.compile("on_main").unwrap();

        let ctx = HookContext::pre_tool_use("Bash").with_git_branch("main");
        assert_eq!(matcher.evaluate(&ctx), Some("on_main"));

        let ctx = HookContext::pre_tool_use("Bash").with_git_branch("feature/x");
        assert_eq!(matcher.evaluate(&ctx), None);

        // No git branch → None → false
        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    // ========== E2E: Combined Conditions (AND) ==========

    #[test]
    fn e2e_combined_event_and_tool() {
        let m = HookMatch {
            event: Some(HookEvent::PreToolUse),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        };
        let matcher = m.compile("pre_bash").unwrap();

        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), Some("pre_bash"));

        // Right tool, wrong event
        let ctx = HookContext::post_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), None);

        // Right event, wrong tool
        let ctx = HookContext::pre_tool_use("Write");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn e2e_combined_all_fields() {
        let m = HookMatch {
            event: Some(HookEvent::PreToolUse),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            arguments: Some(vec![ArgumentMatch {
                name: "command".into(),
                value: StringMatch::Contains("rm".into()),
            }]),
            cwd: Some(StringMatch::Prefix("/home".into())),
            git_branch: Some(StringMatch::Exact("main".into())),
        };
        let matcher = m.compile("full_match").unwrap();

        // All match
        let ctx = HookContext::pre_tool_use("Bash")
            .with_arg("command", "rm -rf /tmp")
            .with_cwd("/home/user")
            .with_git_branch("main");
        assert_eq!(matcher.evaluate(&ctx), Some("full_match"));

        // One field doesn't match (wrong branch)
        let ctx = HookContext::pre_tool_use("Bash")
            .with_arg("command", "rm -rf /tmp")
            .with_cwd("/home/user")
            .with_git_branch("develop");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    // ========== E2E: Empty Match ==========

    #[test]
    fn e2e_empty_match_matches_everything() {
        let m = HookMatch::default();
        let matcher = m.compile("catch_all").unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Bash")),
            Some("catch_all")
        );
        assert_eq!(matcher.evaluate(&HookContext::stop()), Some("catch_all"));
        assert_eq!(
            matcher.evaluate(&HookContext::session_start()),
            Some("catch_all")
        );
    }

    // ========== E2E: Multiple Rules (OR) ==========

    #[test]
    fn e2e_multiple_rules_or() {
        let rules = vec![
            HookMatch {
                tool_name: Some(StringMatch::Exact("Bash".into())),
                ..Default::default()
            },
            HookMatch {
                tool_name: Some(StringMatch::Exact("Write".into())),
                ..Default::default()
            },
        ];

        let matcher = compile_hook_matches(&rules, "blocked", None).unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Bash")),
            Some("blocked")
        );
        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Write")),
            Some("blocked")
        );
        assert_eq!(matcher.evaluate(&HookContext::pre_tool_use("Read")), None);
    }

    #[test]
    fn e2e_multiple_rules_with_fallback() {
        let rules = vec![HookMatch {
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        }];

        let matcher = compile_hook_matches(&rules, "block", Some("allow")).unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Bash")),
            Some("block")
        );
        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("Read")),
            Some("allow")
        );
    }

    #[test]
    fn e2e_empty_rules_matches_everything() {
        let matcher = compile_hook_matches::<&str>(&[], "catch_all", None).unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("anything")),
            Some("catch_all")
        );
    }

    // ========== Error Cases ==========

    #[test]
    fn compile_invalid_regex_returns_error() {
        let m = HookMatch {
            tool_name: Some(StringMatch::Regex("[bad".into())),
            ..Default::default()
        };
        assert!(m.compile::<&str>("x").is_err());
    }

    #[test]
    fn compile_hook_matches_invalid_regex_returns_error() {
        let rules = vec![
            HookMatch::default(),
            HookMatch {
                tool_name: Some(StringMatch::Regex("[bad".into())),
                ..Default::default()
            },
        ];
        assert!(compile_hook_matches::<&str>(&rules, "x", None).is_err());
    }

    // ========== Realistic Scenarios ==========

    #[test]
    fn scenario_block_dangerous_bash() {
        let rule = HookMatch {
            event: Some(HookEvent::PreToolUse),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            arguments: Some(vec![ArgumentMatch {
                name: "command".into(),
                value: StringMatch::Contains("rm -rf".into()),
            }]),
            ..Default::default()
        };
        let matcher = rule.compile("block").unwrap();

        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "rm -rf /important");
        assert_eq!(matcher.evaluate(&ctx), Some("block"));

        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "ls -la");
        assert_eq!(matcher.evaluate(&ctx), None);

        // Not Bash
        let ctx = HookContext::pre_tool_use("Write");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    #[test]
    fn scenario_block_mcp_deletes() {
        let rule = HookMatch {
            tool_name: Some(StringMatch::Regex(r"^mcp__.*__delete".into())),
            ..Default::default()
        };
        let matcher = rule.compile("block").unwrap();

        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("mcp__db__delete_row")),
            Some("block")
        );
        assert_eq!(
            matcher.evaluate(&HookContext::pre_tool_use("mcp__db__query")),
            None
        );
    }

    #[test]
    fn scenario_branch_protection() {
        let rule = HookMatch {
            event: Some(HookEvent::PreToolUse),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            git_branch: Some(StringMatch::Exact("main".into())),
            arguments: Some(vec![ArgumentMatch {
                name: "command".into(),
                value: StringMatch::Contains("push".into()),
            }]),
            ..Default::default()
        };
        let matcher = rule.compile("block_push_to_main").unwrap();

        let ctx = HookContext::pre_tool_use("Bash")
            .with_arg("command", "git push origin main")
            .with_git_branch("main");
        assert_eq!(matcher.evaluate(&ctx), Some("block_push_to_main"));

        // On a feature branch — allowed
        let ctx = HookContext::pre_tool_use("Bash")
            .with_arg("command", "git push origin feat/x")
            .with_git_branch("feat/x");
        assert_eq!(matcher.evaluate(&ctx), None);
    }

    // ========== Trace Tests ==========

    #[test]
    fn trace_all_match() {
        let rule = HookMatch {
            event: Some(HookEvent::PreToolUse),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        };

        let ctx = HookContext::pre_tool_use("Bash");
        let trace = rule.trace(&ctx);

        assert!(trace.matched);
        assert_eq!(trace.steps.len(), 2);
        assert!(trace.steps[0].matched); // event
        assert!(trace.steps[1].matched); // tool_name
    }

    #[test]
    fn trace_partial_match_shows_failure() {
        let rule = HookMatch {
            event: Some(HookEvent::PreToolUse),
            tool_name: Some(StringMatch::Exact("Bash".into())),
            ..Default::default()
        };

        let ctx = HookContext::pre_tool_use("Write");
        let trace = rule.trace(&ctx);

        assert!(!trace.matched);
        assert_eq!(trace.steps.len(), 2);
        assert!(trace.steps[0].matched); // event matches
        assert!(!trace.steps[1].matched); // tool_name doesn't
        assert_eq!(trace.steps[1].field, "tool_name");
        assert_eq!(trace.steps[1].actual, "Write");
    }

    #[test]
    fn trace_missing_argument() {
        let rule = HookMatch {
            arguments: Some(vec![ArgumentMatch {
                name: "command".into(),
                value: StringMatch::Exact("ls".into()),
            }]),
            ..Default::default()
        };

        let ctx = HookContext::pre_tool_use("Bash");
        let trace = rule.trace(&ctx);

        assert!(!trace.matched);
        assert_eq!(trace.steps[0].field, "argument[command]");
        assert_eq!(trace.steps[0].actual, "<missing>");
        assert!(!trace.steps[0].matched);
    }

    #[test]
    fn trace_missing_git_branch() {
        let rule = HookMatch {
            git_branch: Some(StringMatch::Exact("main".into())),
            ..Default::default()
        };

        let ctx = HookContext::pre_tool_use("Bash");
        let trace = rule.trace(&ctx);

        assert!(!trace.matched);
        assert_eq!(trace.steps[0].field, "git_branch");
        assert_eq!(trace.steps[0].actual, "<none>");
        assert!(!trace.steps[0].matched);
    }

    #[test]
    fn trace_empty_rule_has_no_steps() {
        let rule = HookMatch::default();
        let ctx = HookContext::pre_tool_use("Bash");
        let trace = rule.trace(&ctx);

        assert!(trace.matched);
        assert!(trace.steps.is_empty());
    }

    #[test]
    fn trace_shows_expected_values() {
        let rule = HookMatch {
            tool_name: Some(StringMatch::Contains("mcp".into())),
            ..Default::default()
        };

        let ctx = HookContext::pre_tool_use("mcp__db__query");
        let trace = rule.trace(&ctx);

        assert!(trace.matched);
        assert_eq!(trace.steps[0].expected, r#"Contains("mcp")"#);
        assert_eq!(trace.steps[0].actual, "mcp__db__query");
    }
}
