//! Opaque compiled matchers exposed to Python.
//!
//! The key insight: don't expose the Rust type tree across FFI.
//! Config in → compile in Rust → evaluate in Rust → simple types out.

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rumi::claude::{compile_hook_matches, HookContext, HookEvent};
use rumi::prelude::*;

use crate::config::PyHookMatch;
use crate::convert;

/// An opaque compiled hook matcher.
///
/// Created via `HookMatcher.compile()`, immutable after construction.
/// Evaluates Claude Code hook contexts against compiled rules.
///
/// # Thread Safety
///
/// `HookMatcher` is immutable and safe to share across threads.
#[pyclass(frozen)]
pub struct HookMatcher {
    inner: Matcher<HookContext, String>,
}

#[pymethods]
impl HookMatcher {
    /// Compile hook match rules into an immutable matcher.
    ///
    /// # Arguments
    ///
    /// * `rules` — List of `HookMatch` rules (`ORed` together).
    /// * `action` — Action to return when any rule matches.
    /// * `fallback` — Action to return when no rule matches (default: `None`).
    ///
    /// # Errors
    ///
    /// Raises `ValueError` if:
    /// - Any rule has an invalid regex pattern
    /// - An empty rule doesn't have `match_all=True`
    /// - Input limits exceeded (pattern length, argument count, rule count)
    /// - Compiled matcher exceeds depth limit
    #[staticmethod]
    #[pyo3(signature = (rules, action, fallback = None))]
    #[allow(clippy::needless_pass_by_value)] // PyO3 requires owned Vec across FFI
    fn compile(
        rules: Vec<PyHookMatch>,
        action: String,
        fallback: Option<String>,
    ) -> PyResult<Self> {
        // Validate rule count
        if rules.len() > convert::MAX_RULES {
            return Err(PyValueError::new_err(format!(
                "too many rules: {} exceeds limit of {}",
                rules.len(),
                convert::MAX_RULES,
            )));
        }

        // Convert Python rules to Rust HookMatch
        let rust_rules: Vec<_> = rules
            .iter()
            .map(convert::convert_hook_match)
            .collect::<PyResult<_>>()?;

        // Compile via rumi::claude compiler
        let matcher = compile_hook_matches(&rust_rules, action, fallback)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        // Validate depth (Vector: non-negotiable)
        matcher
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self { inner: matcher })
    }

    /// Evaluate a hook context against compiled rules.
    ///
    /// # Arguments
    ///
    /// * `event` — Hook event type (e.g., "`PreToolUse`").
    /// * `tool_name` — Tool name (empty string for non-tool events).
    /// * `arguments` — Tool arguments as key-value pairs.
    /// * `cwd` — Current working directory.
    /// * `session_id` — Session identifier (empty string = anonymous).
    /// * `git_branch` — Git branch name (`None` = not in a repo).
    ///
    /// # Returns
    ///
    /// The action string if rules matched, or `None`.
    #[pyo3(signature = (
        event,
        tool_name = "",
        arguments = None,
        cwd = None,
        session_id = None,
        git_branch = None,
    ))]
    fn evaluate(
        &self,
        event: &str,
        tool_name: &str,
        arguments: Option<HashMap<String, String>>,
        cwd: Option<&str>,
        session_id: Option<&str>,
        git_branch: Option<&str>,
    ) -> PyResult<Option<String>> {
        let ctx = build_context(event, tool_name, arguments, cwd, session_id, git_branch)?;
        Ok(self.inner.evaluate(&ctx))
    }

    /// Trace evaluation for debugging (opt-in, per Vector recommendation).
    ///
    /// Returns the same result as `evaluate()` plus a detailed trace
    /// showing which rules matched and why.
    #[pyo3(signature = (
        event,
        tool_name = "",
        arguments = None,
        cwd = None,
        session_id = None,
        git_branch = None,
    ))]
    fn trace(
        &self,
        event: &str,
        tool_name: &str,
        arguments: Option<HashMap<String, String>>,
        cwd: Option<&str>,
        session_id: Option<&str>,
        git_branch: Option<&str>,
    ) -> PyResult<PyTraceResult> {
        let ctx = build_context(event, tool_name, arguments, cwd, session_id, git_branch)?;
        let trace = self.inner.evaluate_with_trace(&ctx);

        let steps: Vec<PyTraceStep> = trace
            .steps
            .iter()
            .map(|step| PyTraceStep {
                index: step.index,
                matched: step.matched,
                predicate: format!("{:?}", step.predicate_trace),
            })
            .collect();

        Ok(PyTraceResult {
            result: trace.result,
            steps,
            used_fallback: trace.used_fallback,
        })
    }

    #[allow(clippy::unused_self)] // Required by Python __repr__ protocol
    fn __repr__(&self) -> String {
        "HookMatcher(<compiled>)".to_string()
    }
}

/// Build a `HookContext` from Python arguments.
fn build_context(
    event: &str,
    tool_name: &str,
    arguments: Option<HashMap<String, String>>,
    cwd: Option<&str>,
    session_id: Option<&str>,
    git_branch: Option<&str>,
) -> PyResult<HookContext> {
    // Parse event
    let hook_event = match event {
        "PreToolUse" => Ok(HookEvent::PreToolUse),
        "PostToolUse" => Ok(HookEvent::PostToolUse),
        "Stop" => Ok(HookEvent::Stop),
        "SubagentStop" => Ok(HookEvent::SubagentStop),
        "UserPromptSubmit" => Ok(HookEvent::UserPromptSubmit),
        "SessionStart" => Ok(HookEvent::SessionStart),
        "SessionEnd" => Ok(HookEvent::SessionEnd),
        "PreCompact" => Ok(HookEvent::PreCompact),
        "Notification" => Ok(HookEvent::Notification),
        _ => Err(PyValueError::new_err(format!(
            "unknown hook event: {event:?}"
        ))),
    }?;

    // Build context using the builder API
    let mut ctx = match hook_event {
        HookEvent::PreToolUse => HookContext::pre_tool_use(tool_name),
        HookEvent::PostToolUse => HookContext::post_tool_use(tool_name),
        HookEvent::Stop => HookContext::stop(),
        HookEvent::SubagentStop => HookContext::subagent_stop(),
        HookEvent::UserPromptSubmit => HookContext::user_prompt_submit(),
        HookEvent::SessionStart => HookContext::session_start(),
        HookEvent::SessionEnd => HookContext::session_end(),
        HookEvent::PreCompact => HookContext::pre_compact(),
        HookEvent::Notification => HookContext::notification(),
    };

    // Add arguments
    if let Some(args) = arguments {
        for (k, v) in args {
            ctx = ctx.with_arg(k, v);
        }
    }

    // Preserve session_id="" vs git_branch=None semantics (Dijkstra)
    if let Some(sid) = session_id {
        ctx = ctx.with_session_id(sid);
    }
    if let Some(cwd_val) = cwd {
        ctx = ctx.with_cwd(cwd_val);
    }
    if let Some(branch) = git_branch {
        ctx = ctx.with_git_branch(branch);
    }

    Ok(ctx)
}

/// Trace result returned from `HookMatcher.trace()`.
#[pyclass(frozen)]
#[derive(Debug)]
pub struct PyTraceResult {
    /// The action result (same as `evaluate()`).
    #[pyo3(get)]
    pub result: Option<String>,
    /// Trace steps for each field matcher evaluated.
    #[pyo3(get)]
    pub steps: Vec<PyTraceStep>,
    /// Whether the fallback was used.
    #[pyo3(get)]
    pub used_fallback: bool,
}

#[pymethods]
impl PyTraceResult {
    fn __repr__(&self) -> String {
        format!(
            "TraceResult(result={:?}, steps={}, used_fallback={})",
            self.result,
            self.steps.len(),
            self.used_fallback
        )
    }
}

/// One step in the evaluation trace.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PyTraceStep {
    /// Index of the field matcher (0-based).
    #[pyo3(get)]
    pub index: usize,
    /// Whether this predicate matched.
    #[pyo3(get)]
    pub matched: bool,
    /// Debug representation of the predicate trace.
    #[pyo3(get)]
    pub predicate: String,
}

#[pymethods]
impl PyTraceStep {
    fn __repr__(&self) -> String {
        format!(
            "TraceStep(index={}, matched={}, predicate={:?})",
            self.index, self.matched, self.predicate
        )
    }
}
