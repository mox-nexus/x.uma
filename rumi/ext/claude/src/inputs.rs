//! `DataInput` implementations for extracting data from `HookContext`.

use crate::context::HookContext;
use rumi::prelude::*;

/// Extracts the hook event type as a string.
#[derive(Debug, Clone)]
pub struct EventInput;

impl DataInput<HookContext> for EventInput {
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
// Hand-written config types — used when proto feature is not enabled.
// ═══════════════════════════════════════════════════════════════════════════════

/// Configuration for [`ArgumentInput`].
#[cfg(all(feature = "registry", not(feature = "proto")))]
#[derive(serde::Deserialize)]
pub struct ArgumentInputConfig {
    /// The argument name to extract.
    pub name: String,
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HookContext> for EventInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
        Ok(Box::new(EventInput))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HookContext> for ToolNameInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
        Ok(Box::new(ToolNameInput))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HookContext> for ArgumentInput {
    type Config = ArgumentInputConfig;

    fn from_config(
        config: Self::Config,
    ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
        Ok(Box::new(ArgumentInput::new(config.name)))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HookContext> for SessionIdInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
        Ok(Box::new(SessionIdInput))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HookContext> for CwdInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
        Ok(Box::new(CwdInput))
    }
}

#[cfg(all(feature = "registry", not(feature = "proto")))]
impl rumi::IntoDataInput<HookContext> for GitBranchInput {
    type Config = rumi::UnitConfig;

    fn from_config(
        _: rumi::UnitConfig,
    ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
        Ok(Box::new(GitBranchInput))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Proto config types (feature = "proto")
// Uses proto-generated types as Config, enabling xDS control plane integration.
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "proto")]
mod proto_configs {
    use super::*;
    use rumi_proto::xuma::claude::v1 as proto;

    impl rumi::IntoDataInput<HookContext> for EventInput {
        type Config = proto::EventTypeInput;

        fn from_config(
            _: proto::EventTypeInput,
        ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
            Ok(Box::new(EventInput))
        }
    }

    impl rumi::IntoDataInput<HookContext> for ToolNameInput {
        type Config = proto::ToolNameInput;

        fn from_config(
            _: proto::ToolNameInput,
        ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
            Ok(Box::new(ToolNameInput))
        }
    }

    impl rumi::IntoDataInput<HookContext> for ArgumentInput {
        type Config = proto::ToolArgInput;

        fn from_config(
            config: proto::ToolArgInput,
        ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
            Ok(Box::new(ArgumentInput::new(config.name)))
        }
    }

    impl rumi::IntoDataInput<HookContext> for SessionIdInput {
        type Config = proto::SessionIdInput;

        fn from_config(
            _: proto::SessionIdInput,
        ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
            Ok(Box::new(SessionIdInput))
        }
    }

    impl rumi::IntoDataInput<HookContext> for CwdInput {
        type Config = proto::CwdInput;

        fn from_config(
            _: proto::CwdInput,
        ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
            Ok(Box::new(CwdInput))
        }
    }

    impl rumi::IntoDataInput<HookContext> for GitBranchInput {
        type Config = proto::GitBranchInput;

        fn from_config(
            _: proto::GitBranchInput,
        ) -> Result<Box<dyn rumi::DataInput<HookContext>>, rumi::MatcherError> {
            Ok(Box::new(GitBranchInput))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_input_returns_event_string() {
        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(
            EventInput.get(&ctx),
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
                EventInput.get(&ctx),
                MatchingData::String(event_name.into()),
                "EventInput failed for {event_name}"
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
            ArgumentInput::new("command").get(&ctx),
            MatchingData::String("ls".into())
        );
    }

    #[test]
    fn argument_input_returns_none_for_missing() {
        let ctx = HookContext::pre_tool_use("Bash");
        assert_eq!(ArgumentInput::new("command").get(&ctx), MatchingData::None);
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
