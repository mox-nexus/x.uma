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

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        compile_hook_matches, ArgumentInput, ArgumentMatch, CompileError, CwdInput, EventInput,
        GitBranchInput, HookContext, HookEvent, HookMatch, HookMatchExt, HookMatchTrace,
        SessionIdInput, StringMatch, ToolNameInput, TraceStep,
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
