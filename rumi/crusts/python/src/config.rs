//! Typed Python config classes for hook matching.
//!
//! These provide IDE autocomplete and type safety on the Python side,
//! rather than exposing raw dicts (Ace recommendation from guild review).

use pyo3::prelude::*;
use pyo3::Borrowed;

/// How to match a string value.
///
/// Bare strings passed to `HookMatch` fields are treated as exact matches.
#[pyclass(from_py_object)]
#[derive(Debug, Clone)]
pub enum PyStringMatch {
    /// Exact equality.
    Exact { value: String },
    /// Starts with prefix.
    Prefix { value: String },
    /// Ends with suffix.
    Suffix { value: String },
    /// Contains substring.
    Contains { value: String },
    /// Matches regular expression (Rust `regex` crate syntax — linear time).
    Regex { pattern: String },
}

#[pymethods]
impl PyStringMatch {
    /// Create an exact match.
    #[staticmethod]
    fn exact(value: String) -> Self {
        Self::Exact { value }
    }

    /// Create a prefix match.
    #[staticmethod]
    fn prefix(value: String) -> Self {
        Self::Prefix { value }
    }

    /// Create a suffix match.
    #[staticmethod]
    fn suffix(value: String) -> Self {
        Self::Suffix { value }
    }

    /// Create a contains match.
    #[staticmethod]
    fn contains(value: String) -> Self {
        Self::Contains { value }
    }

    /// Create a regex match (Rust `regex` crate — guaranteed linear time).
    #[staticmethod]
    fn regex(pattern: String) -> Self {
        Self::Regex { pattern }
    }

    fn __repr__(&self) -> String {
        match self {
            Self::Exact { value } => format!("StringMatch.exact({value:?})"),
            Self::Prefix { value } => format!("StringMatch.prefix({value:?})"),
            Self::Suffix { value } => format!("StringMatch.suffix({value:?})"),
            Self::Contains { value } => format!("StringMatch.contains({value:?})"),
            Self::Regex { pattern } => format!("StringMatch.regex({pattern:?})"),
        }
    }
}

/// User-friendly configuration for matching Claude Code hook events.
///
/// All fields are optional. Omitted fields match anything.
/// All present fields are `ANDed` (every condition must match).
///
/// # Fail-closed (Vector security requirement)
///
/// An empty `HookMatch` (all fields None) is rejected at compile time
/// unless `match_all=True` is explicitly passed. This prevents accidental
/// catch-all rules from deserialization bugs.
#[pyclass(from_py_object)]
#[derive(Debug, Clone)]
pub struct PyHookMatch {
    pub(crate) event: Option<String>,
    pub(crate) tool_name: Option<PyStringMatch>,
    pub(crate) arguments: Vec<(String, PyStringMatch)>,
    pub(crate) session_id: Option<PyStringMatch>,
    pub(crate) cwd: Option<PyStringMatch>,
    pub(crate) git_branch: Option<PyStringMatch>,
    pub(crate) match_all: bool,
}

#[pymethods]
impl PyHookMatch {
    /// Create a hook match rule.
    ///
    /// Bare strings for `tool_name`, `cwd`, `git_branch` are treated as exact matches.
    #[new]
    #[pyo3(signature = (
        event = None,
        tool_name = None,
        arguments = None,
        session_id = None,
        cwd = None,
        git_branch = None,
        match_all = false,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        event: Option<String>,
        tool_name: Option<PyStringMatchOrStr>,
        arguments: Option<Vec<(String, PyStringMatchOrStr)>>,
        session_id: Option<PyStringMatchOrStr>,
        cwd: Option<PyStringMatchOrStr>,
        git_branch: Option<PyStringMatchOrStr>,
        match_all: bool,
    ) -> Self {
        Self {
            event,
            tool_name: tool_name.map(Into::into),
            arguments: arguments
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            session_id: session_id.map(Into::into),
            cwd: cwd.map(Into::into),
            git_branch: git_branch.map(Into::into),
            match_all,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "HookMatch(event={:?}, tool_name={:?}, match_all={})",
            self.event, self.tool_name, self.match_all
        )
    }
}

/// Accept either `PyStringMatch` or a bare `str` (→ exact match).
///
/// This enables the Ace-recommended convenience: `tool_name="Bash"`.
#[derive(Debug, Clone)]
pub enum PyStringMatchOrStr {
    Match(PyStringMatch),
    Str(String),
}

impl From<PyStringMatchOrStr> for PyStringMatch {
    fn from(v: PyStringMatchOrStr) -> Self {
        match v {
            PyStringMatchOrStr::Match(m) => m,
            PyStringMatchOrStr::Str(s) => PyStringMatch::Exact { value: s },
        }
    }
}

impl<'a, 'py> FromPyObject<'a, 'py> for PyStringMatchOrStr {
    type Error = PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if let Ok(s) = ob.extract::<String>() {
            Ok(Self::Str(s))
        } else {
            Ok(Self::Match(ob.extract::<PyStringMatch>()?))
        }
    }
}
