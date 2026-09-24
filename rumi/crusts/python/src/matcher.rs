//! Opaque compiled matchers exposed to Python.
//!
//! The key insight: don't expose the Rust type tree across FFI.
//! Config in → compile in Rust → evaluate in Rust → simple types out.

use std::collections::HashMap;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;
use rumi_proto::index::ActionIndex;

use crate::protojson::{ActionEntry, Loaded};
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
    inner: Matcher<HookContext, ActionIndex>,
    actions: Actions,
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
        py: Python<'_>,
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

        let matcher = compile_hook_matches(
            &rust_rules,
            ActionIndex(0),
            fallback.as_ref().map(|_| ActionIndex(1)),
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

        matcher
            .validate()
            .map_err(|e| PyValueError::new_err(e.to_string()))?;

        Ok(Self {
            inner: matcher,
            actions: Actions::compiled(py, action, fallback),
        })
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
    // The six hook fields are the API; `py` is PyO3's.
    #[allow(clippy::too_many_arguments)]
    fn evaluate(
        &self,
        py: Python<'_>,
        event: &str,
        tool_name: &str,
        arguments: Option<HashMap<String, String>>,
        cwd: Option<&str>,
        session_id: Option<&str>,
        git_branch: Option<&str>,
    ) -> PyResult<Option<Py<PyString>>> {
        let ctx = build_context(event, tool_name, arguments, cwd, session_id, git_branch)?;
        Ok(self.actions.action(py, self.inner.evaluate(&ctx)))
    }

    /// The decision as an object: `matched`, `action`, `index`, `fallback`, `undecided`.
    #[pyo3(signature = (
        event,
        tool_name = "",
        arguments = None,
        cwd = None,
        session_id = None,
        git_branch = None,
    ))]
    // The six hook fields are the API; `py` is PyO3's.
    #[allow(clippy::too_many_arguments)]
    fn decide(
        &self,
        py: Python<'_>,
        event: &str,
        tool_name: &str,
        arguments: Option<HashMap<String, String>>,
        cwd: Option<&str>,
        session_id: Option<&str>,
        git_branch: Option<&str>,
    ) -> PyResult<PyDecision> {
        let ctx = build_context(event, tool_name, arguments, cwd, session_id, git_branch)?;
        Ok(self.actions.decide(py, self.inner.outcome(&ctx)))
    }

    /// Every action this matcher can return, in index order.
    fn actions(&self) -> Vec<String> {
        self.actions.names()
    }

    /// The metadata carried by the action at `index`.
    fn action_metadata(&self, index: u32) -> PyResult<HashMap<String, String>> {
        self.actions.metadata(index)
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
    // The six hook fields are the API; `py` is PyO3's.
    #[allow(clippy::too_many_arguments)]
    fn trace(
        &self,
        py: Python<'_>,
        event: &str,
        tool_name: &str,
        arguments: Option<HashMap<String, String>>,
        cwd: Option<&str>,
        session_id: Option<&str>,
        git_branch: Option<&str>,
    ) -> PyResult<PyTraceResult> {
        let ctx = build_context(event, tool_name, arguments, cwd, session_id, git_branch)?;
        let trace = self.inner.evaluate_with_trace(&ctx);
        Ok(self.actions.trace(py, &trace))
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

/// The actions a compiled matcher decides between, held host-side (D-056).
///
/// The engine returns an [`ActionIndex`]; the Python string it stands for is
/// made once here, at load, and handed back by reference on every evaluate.
/// Nothing about an action is converted per call.
pub(crate) struct Actions {
    names: Vec<Py<PyString>>,
    entries: Vec<ActionEntry>,
    fallback_start: Option<ActionIndex>,
}

impl Actions {
    fn new(py: Python<'_>, entries: Vec<ActionEntry>, fallback_start: Option<ActionIndex>) -> Self {
        let names = entries
            .iter()
            .map(|e| PyString::new(py, &e.name).unbind())
            .collect();
        Self {
            names,
            entries,
            fallback_start,
        }
    }

    /// The compile door: index 0 is the action, index 1 the fallback.
    pub(crate) fn compiled(py: Python<'_>, action: String, fallback: Option<String>) -> Self {
        let mut entries = vec![ActionEntry::named(action)];
        let fallback_start = fallback.map(|f| {
            entries.push(ActionEntry::named(f));
            ActionIndex(1)
        });
        Self::new(py, entries, fallback_start)
    }

    /// The config door: what `load_indexed` numbered.
    pub(crate) fn split<Ctx>(
        py: Python<'_>,
        loaded: Loaded<Ctx>,
    ) -> (Matcher<Ctx, ActionIndex>, Self) {
        let actions = Self::new(py, loaded.actions, loaded.fallback_start);
        (loaded.matcher, actions)
    }

    pub(crate) fn action(
        &self,
        py: Python<'_>,
        index: Option<ActionIndex>,
    ) -> Option<Py<PyString>> {
        index.map(|i| self.names[i.0 as usize].clone_ref(py))
    }

    fn is_fallback(&self, index: ActionIndex) -> bool {
        self.fallback_start.is_some_and(|start| index >= start)
    }

    pub(crate) fn decide(&self, py: Python<'_>, outcome: rumi::Outcome<ActionIndex>) -> PyDecision {
        let undecided = matches!(outcome, rumi::Outcome::Undecided);
        let index = outcome.into_action();
        PyDecision {
            matched: index.is_some(),
            action: self.action(py, index),
            index: index.map(|i| i.0),
            fallback: index.is_some_and(|i| self.is_fallback(i)),
            undecided,
        }
    }

    pub(crate) fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    pub(crate) fn metadata(&self, index: u32) -> PyResult<HashMap<String, String>> {
        self.entries
            .get(index as usize)
            .map(|e| e.metadata.clone())
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "no action at index {index}; this matcher has {}",
                    self.entries.len()
                ))
            })
    }

    pub(crate) fn trace(
        &self,
        py: Python<'_>,
        trace: &rumi::EvalTrace<ActionIndex>,
    ) -> PyTraceResult {
        PyTraceResult {
            result: self.action(py, trace.result),
            index: trace.result.map(|i| i.0),
            steps: trace_steps(&trace.steps),
            used_fallback: trace.used_fallback,
            undecided: trace.undecided,
        }
    }
}

/// One decision: whether anything matched, the action, its index, whether it
/// came from the root fallback, and whether a required value was absent so
/// nothing was decided (D-069).
#[pyclass(frozen, name = "Decision")]
#[derive(Debug)]
pub struct PyDecision {
    #[pyo3(get)]
    pub matched: bool,
    #[pyo3(get)]
    pub action: Option<Py<PyString>>,
    #[pyo3(get)]
    pub index: Option<u32>,
    #[pyo3(get)]
    pub fallback: bool,
    #[pyo3(get)]
    pub undecided: bool,
}

#[pymethods]
impl PyDecision {
    fn __repr__(&self) -> String {
        format!(
            "Decision(matched={}, action={:?}, index={:?}, fallback={}, undecided={})",
            self.matched, self.action, self.index, self.fallback, self.undecided
        )
    }
}

/// Trace result returned from `HookMatcher.trace()`.
#[pyclass(frozen)]
#[derive(Debug)]
pub struct PyTraceResult {
    /// The action result (same as `evaluate()`).
    #[pyo3(get)]
    pub result: Option<Py<PyString>>,
    /// The action's index, if one was returned.
    #[pyo3(get)]
    pub index: Option<u32>,
    /// Trace steps for each field matcher evaluated.
    #[pyo3(get)]
    pub steps: Vec<PyTraceStep>,
    /// Whether the fallback was used.
    #[pyo3(get)]
    pub used_fallback: bool,
    /// Whether a required value was absent, so nothing was decided (D-069).
    #[pyo3(get)]
    pub undecided: bool,
}

#[pymethods]
impl PyTraceResult {
    fn __repr__(&self) -> String {
        format!(
            "TraceResult(result={:?}, index={:?}, steps={}, used_fallback={}, undecided={})",
            self.result,
            self.index,
            self.steps.len(),
            self.used_fallback,
            self.undecided
        )
    }
}

/// One step in the evaluation trace.
#[pyclass(frozen, from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTraceStep {
    /// Index of the field matcher (0-based). Always 0 for a tree lookup.
    #[pyo3(get)]
    pub index: usize,
    /// Whether this predicate matched.
    #[pyo3(get)]
    pub matched: bool,
    /// Debug representation of the predicate trace.
    #[pyo3(get)]
    pub predicate: String,
}

/// Flatten a trace's steps into the FFI shape.
///
/// A tree performs one lookup rather than a list of predicate evaluations, so
/// it maps to a single step whose `predicate` field describes the lookup. It
/// does **not** synthesize a predicate trace: the trace would then assert a
/// `SinglePredicate` that is not in the config, making it a second and wrong
/// source of truth. One step per entry tried, in the order tried (D-055); a
/// lookup that selected no entry is one unmatched step.
pub fn trace_steps<A: std::fmt::Debug>(steps: &rumi::EvalSteps<A>) -> Vec<PyTraceStep> {
    match steps {
        rumi::EvalSteps::List(list) => list
            .iter()
            .map(|step| PyTraceStep {
                index: step.index,
                matched: step.matched,
                predicate: format!("{:?}", step.predicate_trace),
            })
            .collect(),
        rumi::EvalSteps::Tree(lookup) if lookup.attempts.is_empty() => vec![PyTraceStep {
            index: 0,
            matched: false,
            predicate: format!("{lookup:?}"),
        }],
        rumi::EvalSteps::Tree(lookup) => lookup
            .attempts
            .iter()
            .enumerate()
            .map(|(index, attempt)| PyTraceStep {
                index,
                matched: true,
                predicate: format!("{attempt:?}"),
            })
            .collect(),
    }
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
