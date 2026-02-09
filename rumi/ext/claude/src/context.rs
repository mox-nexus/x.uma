//! Hook event types and context for Claude Code.

use std::collections::HashMap;

/// All Claude Code hook event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEvent {
    /// Before a tool executes.
    PreToolUse,
    /// After a tool completes.
    PostToolUse,
    /// Main agent considers stopping.
    Stop,
    /// Subagent considers stopping.
    SubagentStop,
    /// User submits a prompt.
    UserPromptSubmit,
    /// Session begins.
    SessionStart,
    /// Session ends.
    SessionEnd,
    /// Before context compaction.
    PreCompact,
    /// Notification sent to user.
    Notification,
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
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::SessionStart => "SessionStart",
            Self::SessionEnd => "SessionEnd",
            Self::PreCompact => "PreCompact",
            Self::Notification => "Notification",
        }
    }
}

/// Claude Code hook context for matching.
///
/// Carries all data available at the hook enforcement point:
/// event type, tool information, and session metadata.
#[derive(Debug, Clone)]
pub struct HookContext {
    event: HookEvent,
    tool_name: String,
    arguments: HashMap<String, String>,
    session_id: String,
    cwd: String,
    git_branch: Option<String>,
}

impl HookContext {
    /// Create a `PreToolUse` hook context.
    #[must_use]
    pub fn pre_tool_use(tool_name: impl Into<String>) -> Self {
        Self {
            event: HookEvent::PreToolUse,
            tool_name: tool_name.into(),
            arguments: HashMap::new(),
            session_id: String::new(),
            cwd: String::new(),
            git_branch: None,
        }
    }

    /// Create a `PostToolUse` hook context.
    #[must_use]
    pub fn post_tool_use(tool_name: impl Into<String>) -> Self {
        Self {
            event: HookEvent::PostToolUse,
            tool_name: tool_name.into(),
            arguments: HashMap::new(),
            session_id: String::new(),
            cwd: String::new(),
            git_branch: None,
        }
    }

    /// Create a `Stop` hook context.
    #[must_use]
    pub fn stop() -> Self {
        Self::eventonly(HookEvent::Stop)
    }

    /// Create a `SubagentStop` hook context.
    #[must_use]
    pub fn subagent_stop() -> Self {
        Self::eventonly(HookEvent::SubagentStop)
    }

    /// Create a `UserPromptSubmit` hook context.
    #[must_use]
    pub fn user_prompt_submit() -> Self {
        Self::eventonly(HookEvent::UserPromptSubmit)
    }

    /// Create a `SessionStart` hook context.
    #[must_use]
    pub fn session_start() -> Self {
        Self::eventonly(HookEvent::SessionStart)
    }

    /// Create a `SessionEnd` hook context.
    #[must_use]
    pub fn session_end() -> Self {
        Self::eventonly(HookEvent::SessionEnd)
    }

    /// Create a `PreCompact` hook context.
    #[must_use]
    pub fn pre_compact() -> Self {
        Self::eventonly(HookEvent::PreCompact)
    }

    /// Create a `Notification` hook context.
    #[must_use]
    pub fn notification() -> Self {
        Self::eventonly(HookEvent::Notification)
    }

    /// Internal: create a context with only an event (no tool).
    fn eventonly(event: HookEvent) -> Self {
        Self {
            event,
            tool_name: String::new(),
            arguments: HashMap::new(),
            session_id: String::new(),
            cwd: String::new(),
            git_branch: None,
        }
    }

    /// Add a tool argument (builder pattern).
    #[must_use]
    pub fn with_arg(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.insert(name.into(), value.into());
        self
    }

    /// Set the session ID (builder pattern).
    #[must_use]
    pub fn with_session_id(mut self, id: impl Into<String>) -> Self {
        self.session_id = id.into();
        self
    }

    /// Set the current working directory (builder pattern).
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = cwd.into();
        self
    }

    /// Set the git branch (builder pattern).
    #[must_use]
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Get the hook event type.
    #[must_use]
    pub fn event(&self) -> HookEvent {
        self.event
    }

    /// Get the tool name (empty string for non-tool events).
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.tool_name
    }

    /// Get a tool argument by name.
    #[must_use]
    pub fn argument(&self, name: &str) -> Option<&str> {
        self.arguments.get(name).map(String::as_str)
    }

    /// Get the session ID.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the current working directory.
    #[must_use]
    pub fn cwd(&self) -> &str {
        &self.cwd
    }

    /// Get the git branch (if in a repository).
    #[must_use]
    pub fn git_branch(&self) -> Option<&str> {
        self.git_branch.as_deref()
    }
}
