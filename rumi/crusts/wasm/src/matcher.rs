//! Opaque compiled matchers exposed to TypeScript.
//!
//! The key insight: don't expose the Rust type tree across FFI.
//! Config in → compile in Rust → evaluate in Rust → simple types out.

use rumi::claude::{compile_hook_matches, HookContext, HookEvent};
use rumi::prelude::*;
use rumi_proto::index::ActionIndex;
use wasm_bindgen::prelude::*;

use crate::protojson::{ActionEntry, Loaded};

use crate::convert;

/// An opaque compiled hook matcher.
///
/// Created via `HookMatcher.compile()`, immutable after construction.
/// Evaluates Claude Code hook contexts against compiled rules.
#[wasm_bindgen]
pub struct HookMatcher {
    inner: Matcher<HookContext, ActionIndex>,
    actions: Actions,
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

        let matcher = compile_hook_matches(
            &rust_rules,
            ActionIndex(0),
            fallback.as_ref().map(|_| ActionIndex(1)),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        matcher
            .validate()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Self {
            inner: matcher,
            actions: Actions::compiled(action, fallback),
        })
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
    #[wasm_bindgen(unchecked_return_type = "string | undefined")]
    pub fn evaluate(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let ctx = build_context_from_js(&context)?;
        Ok(self.actions.action(self.inner.evaluate(&ctx)))
    }

    /// The decision as an object: `{ matched, action, index, fallback, undecided }`.
    pub fn decide(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let ctx = build_context_from_js(&context)?;
        Ok(self.actions.decide(self.inner.outcome(&ctx)))
    }

    /// Every action this matcher can return, in index order.
    #[must_use]
    pub fn actions(&self) -> Vec<String> {
        self.actions.names()
    }

    /// The metadata carried by the action at `index`.
    #[wasm_bindgen(js_name = "actionMetadata")]
    pub fn action_metadata(&self, index: u32) -> Result<JsValue, JsValue> {
        self.actions.metadata(index)
    }

    /// Trace evaluation for debugging (opt-in, per Vector recommendation).
    ///
    /// Returns the same result as `evaluate()` plus a detailed trace
    /// showing which rules matched and why.
    pub fn trace(&self, context: JsValue) -> Result<JsValue, JsValue> {
        let ctx = build_context_from_js(&context)?;
        let trace = self.inner.evaluate_with_trace(&ctx);
        self.actions.trace(&trace)
    }
}

/// The actions a compiled matcher decides between, held host-side (D-056).
///
/// The engine returns an [`ActionIndex`]; the JS string it stands for is made
/// once here, at load, and handed back by handle on every evaluate. Nothing
/// about an action is transcoded per call.
pub(crate) struct Actions {
    names: Vec<JsValue>,
    entries: Vec<ActionEntry>,
    fallback_start: Option<ActionIndex>,
}

thread_local! {
    static KEY_MATCHED: JsValue = JsValue::from_str("matched");
    static KEY_ACTION: JsValue = JsValue::from_str("action");
    static KEY_INDEX: JsValue = JsValue::from_str("index");
    static KEY_FALLBACK: JsValue = JsValue::from_str("fallback");
    static KEY_UNDECIDED: JsValue = JsValue::from_str("undecided");
}

impl Actions {
    fn new(entries: Vec<ActionEntry>, fallback_start: Option<ActionIndex>) -> Self {
        let names = entries.iter().map(|e| JsValue::from_str(&e.name)).collect();
        Self {
            names,
            entries,
            fallback_start,
        }
    }

    /// The compile door: index 0 is the action, index 1 the fallback.
    pub(crate) fn compiled(action: String, fallback: Option<String>) -> Self {
        let mut entries = vec![ActionEntry::named(action)];
        let fallback_start = fallback.map(|f| {
            entries.push(ActionEntry::named(f));
            ActionIndex(1)
        });
        Self::new(entries, fallback_start)
    }

    /// The config door: what `load_indexed` numbered.
    pub(crate) fn split<Ctx>(loaded: Loaded<Ctx>) -> (Matcher<Ctx, ActionIndex>, Self) {
        let actions = Self::new(loaded.actions, loaded.fallback_start);
        (loaded.matcher, actions)
    }

    pub(crate) fn action(&self, index: Option<ActionIndex>) -> JsValue {
        index.map_or(JsValue::UNDEFINED, |i| self.names[i.0 as usize].clone())
    }

    fn is_fallback(&self, index: ActionIndex) -> bool {
        self.fallback_start.is_some_and(|start| index >= start)
    }

    pub(crate) fn decide(&self, outcome: rumi::Outcome<ActionIndex>) -> JsValue {
        let undecided = matches!(outcome, rumi::Outcome::Undecided);
        let index = outcome.into_action();
        let obj = js_sys::Object::new();
        let set = |key: &'static std::thread::LocalKey<JsValue>, value: JsValue| {
            key.with(|k| {
                js_sys::Reflect::set(&obj, k, &value).expect("a fresh object accepts a property");
            });
        };
        set(&KEY_MATCHED, JsValue::from_bool(index.is_some()));
        set(&KEY_ACTION, self.action(index));
        set(
            &KEY_INDEX,
            index.map_or(JsValue::UNDEFINED, |i| JsValue::from_f64(f64::from(i.0))),
        );
        set(
            &KEY_FALLBACK,
            JsValue::from_bool(index.is_some_and(|i| self.is_fallback(i))),
        );
        set(&KEY_UNDECIDED, JsValue::from_bool(undecided));
        obj.into()
    }

    pub(crate) fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    pub(crate) fn metadata(&self, index: u32) -> Result<JsValue, JsValue> {
        let entry = self.entries.get(index as usize).ok_or_else(|| {
            JsValue::from_str(&format!(
                "no action at index {index}; this matcher has {}",
                self.entries.len()
            ))
        })?;
        // A plain object, not a `Map`: metadata is read like any other record.
        let plain = serde_wasm_bindgen::Serializer::json_compatible();
        serde::Serialize::serialize(&entry.metadata, &plain)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub(crate) fn trace(&self, trace: &rumi::EvalTrace<ActionIndex>) -> Result<JsValue, JsValue> {
        let result = TraceResultSerde {
            result: trace
                .result
                .map(|i| self.entries[i.0 as usize].name.clone()),
            index: trace.result.map(|i| i.0),
            steps: trace_steps(&trace.steps),
            used_fallback: trace.used_fallback,
            undecided: trace.undecided,
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
    pub(crate) index: Option<u32>,
    pub(crate) steps: Vec<TraceStepSerde>,
    pub(crate) used_fallback: bool,
    pub(crate) undecided: bool,
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
