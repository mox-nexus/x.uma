//! rumi-claude: Claude Code hooks domain
//!
//! Provides context and `DataInput` implementations for Claude Code hook matching.
//!
//! # Example
//!
//! ```ignore
//! use rumi_claude::prelude::*;
//!
//! let ctx = HookContext::pre_tool_use("Bash")
//!     .with_arg("command", "rm -rf /");
//!
//! let input = ToolNameInput;
//! assert_eq!(input.get(&ctx), MatchingData::String("Bash".into()));
//! ```

use rumi::prelude::*;
use std::collections::HashMap;

/// Hook event types in Claude Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    Stop,
    SubagentStop,
}

impl HookEvent {
    /// Convert to string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::Stop => "Stop",
            Self::SubagentStop => "SubagentStop",
        }
    }
}

/// Claude Code hook context for matching.
#[derive(Debug, Clone)]
pub struct HookContext {
    event: HookEvent,
    tool_name: String,
    arguments: HashMap<String, String>,
}

impl HookContext {
    /// Create a `PreToolUse` hook context.
    #[must_use]
    pub fn pre_tool_use(tool_name: impl Into<String>) -> Self {
        Self {
            event: HookEvent::PreToolUse,
            tool_name: tool_name.into(),
            arguments: HashMap::new(),
        }
    }

    /// Create a `PostToolUse` hook context.
    #[must_use]
    pub fn post_tool_use(tool_name: impl Into<String>) -> Self {
        Self {
            event: HookEvent::PostToolUse,
            tool_name: tool_name.into(),
            arguments: HashMap::new(),
        }
    }

    /// Add an argument (builder pattern).
    #[must_use]
    pub fn with_arg(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.insert(name.into(), value.into());
        self
    }

    /// Get the hook event type.
    #[must_use]
    pub fn event(&self) -> HookEvent {
        self.event
    }

    /// Get the tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Get an argument by name.
    #[must_use]
    pub fn argument(&self, name: &str) -> Option<&str> {
        self.arguments.get(name).map(String::as_str)
    }
}

/// Extracts the hook event type.
#[derive(Debug, Clone)]
pub struct EventInput;

impl DataInput<HookContext> for EventInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        MatchingData::String(ctx.event.as_str().to_string())
    }
}

/// Extracts the tool name.
#[derive(Debug, Clone)]
pub struct ToolNameInput;

impl DataInput<HookContext> for ToolNameInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        MatchingData::String(ctx.tool_name.clone())
    }
}

/// Extracts a tool argument by name.
#[derive(Debug, Clone)]
pub struct ArgumentInput {
    name: String,
}

impl ArgumentInput {
    /// Create a new argument input extractor.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl DataInput<HookContext> for ArgumentInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        ctx.argument(&self.name)
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{ArgumentInput, EventInput, HookContext, HookEvent, ToolNameInput};
    pub use rumi::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

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
