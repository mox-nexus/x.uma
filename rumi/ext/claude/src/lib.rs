//! rumi-claude: Claude Code hooks domain
//!
//! Provides context types, `DataInput` extractors, and a **domain compiler**
//! for matching Claude Code hook events.
//!
//! # Compiler (recommended)
//!
//! ```ignore
//! use rumi_claude::prelude::*;
//!
//! // Declarative: block dangerous Bash commands
//! let rule = HookMatch {
//!     event: Some(HookEvent::PreToolUse),
//!     tool_name: Some(StringMatch::Exact("Bash".into())),
//!     arguments: Some(vec![ArgumentMatch {
//!         name: "command".into(),
//!         value: StringMatch::Contains("rm -rf".into()),
//!     }]),
//!     ..Default::default()
//! };
//! let matcher = rule.compile("block")?;
//!
//! let ctx = HookContext::pre_tool_use("Bash")
//!     .with_arg("command", "rm -rf /important");
//! assert_eq!(matcher.evaluate(&ctx), Some("block"));
//! ```
//!
//! # Trace (debugging)
//!
//! ```ignore
//! let trace = rule.trace(&ctx);
//! for step in &trace.steps {
//!     println!("{}: expected={}, actual={}, matched={}",
//!         step.field, step.expected, step.actual, step.matched);
//! }
//! ```

mod compiler;
mod config;
mod context;
mod inputs;

pub use compiler::*;
pub use config::*;
pub use context::*;
pub use inputs::*;

// Registry config types (hand-written, only without proto)
#[cfg(all(feature = "registry", not(feature = "proto")))]
pub use inputs::ArgumentInputConfig;

/// Register all rumi-claude types for [`HookContext`] with the given builder.
///
/// Registers core matchers (`BoolMatcher`, `StringMatcher`) and Claude-domain inputs:
/// - `xuma.claude.v1.EventInput` → [`EventInput`]
/// - `xuma.claude.v1.ToolNameInput` → [`ToolNameInput`]
/// - `xuma.claude.v1.ArgumentInput` → [`ArgumentInput`]
/// - `xuma.claude.v1.SessionIdInput` → [`SessionIdInput`]
/// - `xuma.claude.v1.CwdInput` → [`CwdInput`]
/// - `xuma.claude.v1.GitBranchInput` → [`GitBranchInput`]
#[cfg(feature = "registry")]
#[must_use]
pub fn register(builder: rumi::RegistryBuilder<HookContext>) -> rumi::RegistryBuilder<HookContext> {
    rumi::register_core_matchers(builder)
        .input::<EventInput>("xuma.claude.v1.EventInput")
        .input::<ToolNameInput>("xuma.claude.v1.ToolNameInput")
        .input::<ArgumentInput>("xuma.claude.v1.ArgumentInput")
        .input::<SessionIdInput>("xuma.claude.v1.SessionIdInput")
        .input::<CwdInput>("xuma.claude.v1.CwdInput")
        .input::<GitBranchInput>("xuma.claude.v1.GitBranchInput")
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        compile_hook_matches, ArgumentInput, ArgumentMatch, CwdInput, EventInput, GitBranchInput,
        HookContext, HookEvent, HookMatch, HookMatchExt, HookMatchTrace, SessionIdInput,
        StringMatch, ToolNameInput, TraceStep,
    };
    pub use rumi::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rumi::prelude::*;

    #[test]
    fn test_hook_context_builder() {
        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "ls -la");

        assert_eq!(ctx.event(), HookEvent::PreToolUse);
        assert_eq!(ctx.tool_name(), "Bash");
        assert_eq!(ctx.argument("command"), Some("ls -la"));
    }

    #[test]
    fn test_tool_name_input() {
        let ctx = HookContext::pre_tool_use("Write");
        assert_eq!(
            ToolNameInput.get(&ctx),
            MatchingData::String("Write".into())
        );
    }

    #[test]
    fn test_event_input() {
        let ctx = HookContext::post_tool_use("Read");
        assert_eq!(
            EventInput.get(&ctx),
            MatchingData::String("PostToolUse".into())
        );
    }

    #[test]
    fn test_argument_input() {
        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "echo hello");

        let input = ArgumentInput::new("command");
        assert_eq!(input.get(&ctx), MatchingData::String("echo hello".into()));
    }

    #[test]
    fn test_dangerous_command_matcher() {
        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "rm -rf /important");

        // Match dangerous Bash commands
        let matcher: Matcher<HookContext, &str> = Matcher::new(
            vec![FieldMatcher::new(
                Predicate::And(vec![
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ToolNameInput),
                        Box::new(ExactMatcher::new("Bash")),
                    )),
                    Predicate::Single(SinglePredicate::new(
                        Box::new(ArgumentInput::new("command")),
                        Box::new(ContainsMatcher::new("rm -rf")),
                    )),
                ]),
                OnMatch::Action("block"),
            )],
            Some(OnMatch::Action("allow")),
        );

        assert_eq!(matcher.evaluate(&ctx), Some("block"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Proto registry integration tests
// Verifies: proto config → register() → load_matcher → evaluate on HookContext
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(all(test, feature = "proto"))]
mod proto_tests {
    use super::*;
    use rumi::MatcherConfig;

    #[test]
    fn register_builds_with_proto_configs() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        assert!(registry.contains_input("xuma.claude.v1.EventInput"));
        assert!(registry.contains_input("xuma.claude.v1.ToolNameInput"));
        assert!(registry.contains_input("xuma.claude.v1.ArgumentInput"));
        assert!(registry.contains_input("xuma.claude.v1.SessionIdInput"));
        assert!(registry.contains_input("xuma.claude.v1.CwdInput"));
        assert!(registry.contains_input("xuma.claude.v1.GitBranchInput"));
        assert!(registry.contains_matcher("xuma.core.v1.StringMatcher"));
    }

    #[test]
    fn load_matcher_with_proto_tool_name_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // ToolNameInput is an empty proto — no config fields
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.claude.v1.ToolNameInput",
                        "config": {}
                    },
                    "value_match": { "Exact": "Bash" }
                },
                "on_match": { "type": "action", "action": "is_bash" }
            }],
            "on_no_match": { "type": "action", "action": "not_bash" }
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), Some("is_bash".to_string()));

        let ctx = HookContext::pre_tool_use("Write");
        assert_eq!(matcher.evaluate(&ctx), Some("not_bash".to_string()));
    }

    #[test]
    fn load_matcher_with_proto_argument_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        // ToolArgInput config has "name" field
        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "and",
                    "predicates": [
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.claude.v1.ToolNameInput",
                                "config": {}
                            },
                            "value_match": { "Exact": "Bash" }
                        },
                        {
                            "type": "single",
                            "input": {
                                "type_url": "xuma.claude.v1.ArgumentInput",
                                "config": { "name": "command" }
                            },
                            "value_match": { "Contains": "rm -rf" }
                        }
                    ]
                },
                "on_match": { "type": "action", "action": "block" }
            }],
            "on_no_match": { "type": "action", "action": "allow" }
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "rm -rf /");
        assert_eq!(matcher.evaluate(&ctx), Some("block".to_string()));

        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "ls -la");
        assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));

        let ctx = HookContext::pre_tool_use("Write");
        assert_eq!(matcher.evaluate(&ctx), Some("allow".to_string()));
    }

    #[test]
    fn load_matcher_with_proto_event_input() {
        let registry = register(rumi::RegistryBuilder::new()).build();

        let json = serde_json::json!({
            "matchers": [{
                "predicate": {
                    "type": "single",
                    "input": {
                        "type_url": "xuma.claude.v1.EventInput",
                        "config": {}
                    },
                    "value_match": { "Exact": "PreToolUse" }
                },
                "on_match": { "type": "action", "action": "pre_tool" }
            }]
        });

        let config: MatcherConfig<String> = serde_json::from_value(json).unwrap();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), Some("pre_tool".to_string()));

        let ctx = HookContext::post_tool_use("Bash");
        assert_eq!(matcher.evaluate(&ctx), None);
    }
}
