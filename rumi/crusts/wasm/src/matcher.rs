//! Opaque compiled matchers exposed to TypeScript.
//!
//! The key insight: don't expose the Rust type tree across FFI.
//! Config in → compile in Rust → evaluate in Rust → simple types out.

use rumi::claude::{compile_hook_matches, HookContext, HookEvent};
use rumi::prelude::*;
use wasm_bindgen::prelude::*;

use crate::convert;

/// An opaque compiled hook matcher.
///
/// Created via `HookMatcher.compile()`, immutable after construction.
/// Evaluates Claude Code hook contexts against compiled rules.
#[wasm_bindgen]
pub struct HookMatcher {
    inner: Matcher<HookContext, String>,
}

#[wasm_bindgen]
#[allow(clippy::needless_pass_by_value)] // wasm-bindgen requires owned JsValue across FFI
impl HookMatcher {
    /// Compile hook match rules into an immutable matcher.
    ///
    /// # Arguments
    ///
    /// * `rules` — Array of plain objects (`ORed` together). Each object may have:
    ///   - `event?: string` — Hook event type
    ///   - `toolName?: string | StringMatch` — Tool name (bare string = exact match)
    ///   - `arguments?: Array<[string, string | StringMatch]>` — Argument matchers
    ///   - `cwd?: string | StringMatch` — Working directory
    ///   - `gitBranch?: string | StringMatch` — Git branch
    ///   - `matchAll?: boolean` — Allow empty catch-all (fail-closed guard)
    /// * `action` — Action to return when any rule matches.
    /// * `fallback` — Action to return when no rule matches (default: `undefined`).
    ///
    /// # Errors
    ///
    /// Throws if:
    /// - Any rule has an invalid regex pattern
    /// - An empty rule doesn't have `matchAll: true`
    /// - Input limits exceeded (pattern length, argument count, rule count)
    /// - Compiled matcher exceeds depth limit
    pub fn compile(
        rules: JsValue,
        action: String,
        fallback: Option<String>,
    ) -> Result<HookMatcher, JsValue> {
        let rules_array = js_sys::Array::from(&rules);

        // Validate rule count
        if rules_array.length() > convert::MAX_RULES {
            return Err(JsValue::from_str(&format!(
                "too many rules: {} exceeds limit of {}",
                rules_array.length(),
                convert::MAX_RULES,
            )));
        }

        // Convert JS objects to Rust HookMatch
        let mut rust_rules = Vec::with_capacity(rules_array.length() as usize);
        for rule in rules_array.iter() {
            rust_rules.push(convert::convert_hook_match_from_js(&rule)?);
        }

        // Compile via rumi::claude compiler
        let matcher = compile_hook_matches(&rust_rules, action, fallback)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Validate depth (Vector: non-negotiable)
        matcher
            .validate()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self { inner: matcher })
    }

    /// Evaluate a hook context against compiled rules.
    ///
    /// Accepts a plain object with camelCase fields:
    /// ```js
    /// matcher.evaluate({
    ///   event: "PreToolUse",
    ///   toolName: "Bash",
    ///   arguments: { command: "rm -rf /" },
    ///   cwd: "/home/user",
    ///   sessionId: "abc123",
    ///   gitBranch: "main",
    /// })
    /// ```
    ///
    /// Returns the action string if rules matched, or `undefined`.
    pub fn evaluate(&self, context: JsValue) -> Result<Option<String>, JsValue> {
        let ctx = build_context_from_js(&context)?;
        Ok(self.inner.evaluate(&ctx))
    }

    /// Trace evaluation for debugging (opt-in, per Vector recommendation).
    ///
    /// Returns the same result as `evaluate()` plus a detailed trace
    /// showing which rules matched and why.
    pub fn trace(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let ctx = build_context_from_js(&context)?;
        let trace = self.inner.evaluate_with_trace(&ctx);

        let steps = trace_steps(&trace.steps);

        let result = TraceResultSerde {
            result: trace.result,
            steps,
            used_fallback: trace.used_fallback,
        };

        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

thread_local! {
    // The six field names this module probes, converted to JsValue once per
    // thread rather than once per call.
    //
    // `JsValue::from_str` allocates a fresh JS string and transcodes UTF-8 to
    // UTF-16 across the boundary every time it runs. These names are compile-time
    // constants, and `build_context_from_js` probed all six unconditionally on
    // every `evaluate()` — five of them usually returning `undefined`. Measured
    // 2026-09-06: `evaluate` on a one-rule matcher cost ~1700ns against ~40ns of
    // actual engine work, and the KV path, which does no `from_str` and no
    // `Reflect::get` at all, ran the same engine in ~738ns. That ~960ns delta
    // across six fields is this.
    //
    // Interning (the `enable-interning` feature, now on) covers strings that
    // cross through the generated glue; it does not cover `JsValue::from_str`,
    // which goes straight to `__wbindgen_string_new`. So the keys are cached here
    // explicitly rather than relying on the feature to catch them.
    static KEY_EVENT: JsValue = JsValue::from_str("event");
    static KEY_TOOL_NAME: JsValue = JsValue::from_str("toolName");
    static KEY_ARGUMENTS: JsValue = JsValue::from_str("arguments");
    static KEY_SESSION_ID: JsValue = JsValue::from_str("sessionId");
    static KEY_CWD: JsValue = JsValue::from_str("cwd");
    static KEY_GIT_BRANCH: JsValue = JsValue::from_str("gitBranch");
}

/// Read one field off a JS object using a cached key.
fn get_field(val: &JsValue, key: &'static std::thread::LocalKey<JsValue>) -> JsValue {
    key.with(|k| js_sys::Reflect::get(val, k).unwrap_or(JsValue::UNDEFINED))
}

/// Build a `HookContext` from a JS options object.
fn build_context_from_js(val: &JsValue) -> Result<HookContext, JsValue> {
    // Parse event (required)
    let event_val = get_field(val, &KEY_EVENT);
    let event_str = event_val
        .as_string()
        .ok_or_else(|| JsValue::from_str("event is required and must be a string"))?;
    let hook_event = convert::parse_hook_event(&event_str)?;

    // Tool name (optional, default "")
    let tool_name_val = get_field(val, &KEY_TOOL_NAME);
    let tool_name = tool_name_val.as_string().unwrap_or_default();

    // Build context
    let mut ctx = match hook_event {
        HookEvent::PreToolUse => HookContext::pre_tool_use(&tool_name),
        HookEvent::PostToolUse => HookContext::post_tool_use(&tool_name),
        HookEvent::Stop => HookContext::stop(),
        HookEvent::SubagentStop => HookContext::subagent_stop(),
        HookEvent::UserPromptSubmit => HookContext::user_prompt_submit(),
        HookEvent::SessionStart => HookContext::session_start(),
        HookEvent::SessionEnd => HookContext::session_end(),
        HookEvent::PreCompact => HookContext::pre_compact(),
        HookEvent::Notification => HookContext::notification(),
    };

    // Arguments (optional, Record<string, string>)
    let args_val = get_field(val, &KEY_ARGUMENTS);
    if !args_val.is_undefined() && !args_val.is_null() {
        let entries = js_sys::Object::entries(&js_sys::Object::from(args_val));
        for entry in entries.iter() {
            let pair = js_sys::Array::from(&entry);
            let key = pair
                .get(0)
                .as_string()
                .ok_or_else(|| JsValue::from_str("argument key must be a string"))?;
            let value = pair
                .get(1)
                .as_string()
                .ok_or_else(|| JsValue::from_str("argument value must be a string"))?;
            ctx = ctx.with_arg(key, value);
        }
    }

    // Preserve session_id="" vs git_branch=None semantics (Dijkstra)
    let sid_val = get_field(val, &KEY_SESSION_ID);
    if let Some(sid) = sid_val.as_string() {
        ctx = ctx.with_session_id(&sid);
    }

    let cwd_val = get_field(val, &KEY_CWD);
    if let Some(cwd) = cwd_val.as_string() {
        ctx = ctx.with_cwd(&cwd);
    }

    let branch_val = get_field(val, &KEY_GIT_BRANCH);
    if let Some(branch) = branch_val.as_string() {
        ctx = ctx.with_git_branch(&branch);
    }

    Ok(ctx)
}

/// Serde-serializable trace result (for serde-wasm-bindgen output).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TraceResultSerde {
    pub(crate) result: Option<String>,
    pub(crate) steps: Vec<TraceStepSerde>,
    pub(crate) used_fallback: bool,
}

/// One step in the evaluation trace.
#[derive(serde::Serialize)]
pub(crate) struct TraceStepSerde {
    pub(crate) index: usize,
    pub(crate) matched: bool,
    pub(crate) predicate: String,
}

/// Flatten a trace's steps into the FFI shape.
///
/// A tree performs one lookup rather than a list of predicate evaluations, so
/// it maps to a single step whose `predicate` field describes the lookup. It
/// does **not** synthesize a predicate trace: the trace would then assert a
/// `SinglePredicate` that is not in the config, making it a second and wrong
/// source of truth. One step per entry tried, in the order tried (D-055); a
/// lookup that selected no entry is one unmatched step.
pub(crate) fn trace_steps<A: std::fmt::Debug>(steps: &rumi::EvalSteps<A>) -> Vec<TraceStepSerde> {
    match steps {
        rumi::EvalSteps::List(list) => list
            .iter()
            .map(|step| TraceStepSerde {
                index: step.index,
                matched: step.matched,
                predicate: format!("{:?}", step.predicate_trace),
            })
            .collect(),
        rumi::EvalSteps::Tree(lookup) if lookup.attempts.is_empty() => vec![TraceStepSerde {
            index: 0,
            matched: false,
            predicate: format!("{lookup:?}"),
        }],
        rumi::EvalSteps::Tree(lookup) => lookup
            .attempts
            .iter()
            .enumerate()
            .map(|(index, attempt)| TraceStepSerde {
                index,
                matched: true,
                predicate: format!("{attempt:?}"),
            })
            .collect(),
    }
}
