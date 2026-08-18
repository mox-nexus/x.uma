//! `DataInput` implementations for extracting data from `HookContext`.

use super::context::HookContext;
use crate::prelude::*;

/// Extracts the hook event type as a string.
#[derive(Debug, Clone)]
pub struct EventTypeInput;

impl DataInput<HookContext> for EventTypeInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        MatchingData::String(ctx.event().as_str().to_string())
    }
}

/// Extracts the tool name.
#[derive(Debug, Clone)]
pub struct ToolNameInput;

impl DataInput<HookContext> for ToolNameInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        MatchingData::String(ctx.tool_name().to_string())
    }
}

/// Extracts a tool argument by name.
#[derive(Debug, Clone)]
pub struct ToolArgInput {
    name: String,
}

impl ToolArgInput {
    /// Create a new argument input extractor.
    ///
    /// # Errors
    ///
    /// [`MatcherError::EmptyIdentifier`](crate::MatcherError::EmptyIdentifier)
    /// if `name` is empty. This input names which tool argument to read, and
    /// this is the path that gates agent tool calls — an empty name reads
    /// nothing, so the rule stops firing and the call proceeds.
    pub fn new(name: impl Into<String>) -> Result<Self, crate::MatcherError> {
        let name = name.into();
        if name.is_empty() {
            return Err(crate::MatcherError::EmptyIdentifier {
                what: "tool argument name",
            });
        }
        Ok(Self { name })
    }
}

impl DataInput<HookContext> for ToolArgInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        ctx.argument(&self.name)
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

/// Extracts the session ID.
#[derive(Debug, Clone)]
pub struct SessionIdInput;

impl DataInput<HookContext> for SessionIdInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        MatchingData::String(ctx.session_id().to_string())
    }
}

/// Extracts the current working directory.
#[derive(Debug, Clone)]
pub struct CwdInput;

impl DataInput<HookContext> for CwdInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        MatchingData::String(ctx.cwd().to_string())
    }
}

/// Extracts the git branch, or `None` if not in a repository.
#[derive(Debug, Clone)]
pub struct GitBranchInput;

impl DataInput<HookContext> for GitBranchInput {
    fn get(&self, ctx: &HookContext) -> MatchingData {
        ctx.git_branch()
            .map_or(MatchingData::None, |s| MatchingData::String(s.to_string()))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registry support (feature = "registry")
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for [`ToolArgInput`].
#[cfg(feature = "registry")]
#[derive(serde::Deserialize)]
pub struct ToolArgInputConfig {
    /// The argument name to extract.
    pub name: String,
}

#[cfg(feature = "registry")]
impl crate::IntoDataInput<HookContext> for EventTypeInput {
    type Config = crate::UnitConfig;

    fn from_config(
        _: crate::UnitConfig,
    ) -> Result<Box<dyn crate::DataInput<HookContext>>, crate::MatcherError> {
        Ok(Box::new(EventTypeInput))
    }
}

#[cfg(feature = "registry")]
impl crate::IntoDataInput<HookContext> for ToolNameInput {
    type Config = crate::UnitConfig;

    fn from_config(
        _: crate::UnitConfig,
    ) -> Result<Box<dyn crate::DataInput<HookContext>>, crate::MatcherError> {
        Ok(Box::new(ToolNameInput))
    }
}

#[cfg(feature = "registry")]
impl crate::IntoDataInput<HookContext> for ToolArgInput {
    type Config = ToolArgInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn crate::DataInput<HookContext>>, crate::MatcherError> {
        Ok(Box::new(ToolArgInput::new(config.name)?))
    }
}

#[cfg(feature = "registry")]
impl crate::IntoDataInput<HookContext> for SessionIdInput {
    type Config = crate::UnitConfig;

    fn from_config(
        _: crate::UnitConfig,
    ) -> Result<Box<dyn crate::DataInput<HookContext>>, crate::MatcherError> {
        Ok(Box::new(SessionIdInput))
    }
}

#[cfg(feature = "registry")]
impl crate::IntoDataInput<HookContext> for CwdInput {
    type Config = crate::UnitConfig;

    fn from_config(
        _: crate::UnitConfig,
    ) -> Result<Box<dyn crate::DataInput<HookContext>>, crate::MatcherError> {
        Ok(Box::new(CwdInput))
    }
}

#[cfg(feature = "registry")]
impl crate::IntoDataInput<HookContext> for GitBranchInput {
    type Config = crate::UnitConfig;

    fn from_config(
        _: crate::UnitConfig,
    ) -> Result<Box<dyn crate::DataInput<HookContext>>, crate::MatcherError> {
        Ok(Box::new(GitBranchInput))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This is the input that names which tool argument a gate reads. Empty, it
    /// reads nothing, the rule stops firing, and the tool call proceeds.
    #[test]
    fn an_empty_argument_name_is_rejected() {
        let err = ToolArgInput::new("").unwrap_err();
        assert!(
            matches!(err, crate::MatcherError::EmptyIdentifier { .. }),
            "{err:?}"
        );
    }
    #[test]
    fn event_input_returns_event_string() {
        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(
            EventTypeInput.get(&ctx),
            MatchingData::String("PreToolUse".into())
        );
    }

    #[test]
    fn event_input_all_variants() {
        for (event_name, ctx) in [
            ("PreToolUse", HookContext::pre_tool_use("X")),
            ("PostToolUse", HookContext::post_tool_use("X")),
            ("Stop", HookContext::stop()),
            ("SubagentStop", HookContext::subagent_stop()),
            ("UserPromptSubmit", HookContext::user_prompt_submit()),
            ("SessionStart", HookContext::session_start()),
            ("SessionEnd", HookContext::session_end()),
            ("PreCompact", HookContext::pre_compact()),
            ("Notification", HookContext::notification()),
        ] {
            assert_eq!(
                EventTypeInput.get(&ctx),
                MatchingData::String(event_name.into()),
                "EventTypeInput failed for {event_name}"
            );
        }
    }

    #[test]
    fn tool_name_input_returns_tool() {
        let ctx = HookContext::pre_tool_use("Write");
        assert_eq!(
            ToolNameInput.get(&ctx),
            MatchingData::String("Write".into())
        );
    }

    #[test]
    fn tool_name_input_empty_for_non_tool_events() {
        let ctx = HookContext::stop();
        assert_eq!(ToolNameInput.get(&ctx), MatchingData::String(String::new()));
    }

    #[test]
    fn argument_input_returns_value() {
        let ctx = HookContext::pre_tool_use("Bash").with_arg("command", "ls");
        assert_eq!(
            ToolArgInput::new("command").unwrap().get(&ctx),
            MatchingData::String("ls".into())
        );
    }

    #[test]
    fn argument_input_returns_none_for_missing() {
        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(
            ToolArgInput::new("command").unwrap().get(&ctx),
            MatchingData::None
        );
    }

    #[test]
    fn session_id_input() {
        let ctx = HookContext::pre_tool_use("Bash").with_session_id("abc-123");
        assert_eq!(
            SessionIdInput.get(&ctx),
            MatchingData::String("abc-123".into())
        );
    }

    #[test]
    fn cwd_input() {
        let ctx = HookContext::pre_tool_use("Bash").with_cwd("/home/user/project");
        assert_eq!(
            CwdInput.get(&ctx),
            MatchingData::String("/home/user/project".into())
        );
    }

    #[test]
    fn git_branch_input_present() {
        let ctx = HookContext::pre_tool_use("Bash").with_git_branch("main");
        assert_eq!(
            GitBranchInput.get(&ctx),
            MatchingData::String("main".into())
        );
    }

    #[test]
    fn git_branch_input_absent() {
        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(GitBranchInput.get(&ctx), MatchingData::None);
    }
}
