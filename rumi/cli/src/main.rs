//! rumi CLI — driving adapter for the rumi matcher engine.
//!
//! Supports three domains:
//! - **test** (default) — key-value context pairs
//! - **http** — HTTP method, path, headers, query params
//! - **claude** — Claude Code hook events
//!
//! Subcommands:
//! - `run [http|claude] <config> [flags]` — run config against context
//! - `check [http|claude] <config>` — validate config loads without errors
//! - `info [http|claude]` — print registered type URLs

use std::collections::HashMap;
use std::process;

use rumi::claude::HookContext;
use rumi::MatcherConfig;
use rumi_http::HttpRequest;
use rumi_test::TestContext;

mod hook;
mod skill;
mod trace_output;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let result = match args[1].as_str() {
        "run" => cmd_run(&args[2..]),
        "check" => cmd_check(&args[2..]),
        "info" => cmd_info(&args[2..]),
        "--skill" => cmd_skill(&args[2..]),
        "--help" | "-h" | "help" => {
            print_usage();
            Ok(())
        }
        other => {
            eprintln!("error: unknown command \"{other}\"");
            print_usage();
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Domain detection
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Domain {
    Test,
    Http,
    Claude,
}

/// Detect the domain from the first argument. Returns `(domain, remaining_args)`.
fn detect_domain(args: &[String]) -> (Domain, &[String]) {
    match args.first().map(String::as_str) {
        Some("http") => (Domain::Http, &args[1..]),
        Some("claude") => (Domain::Claude, &args[1..]),
        _ => (Domain::Test, args),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Commands
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_run(args: &[String]) -> Result<(), String> {
    let (domain, rest) = detect_domain(args);
    match domain {
        Domain::Test => cmd_run_test(rest),
        Domain::Http => cmd_run_http(rest),
        Domain::Claude => cmd_run_claude(rest),
    }
}

fn cmd_check(args: &[String]) -> Result<(), String> {
    let (domain, rest) = detect_domain(args);

    if rest.is_empty() {
        return Err("check requires a config file path".into());
    }

    let config_path = &rest[0];
    let config = load_config(config_path)?;

    // Summarise before loading: the loaded Matcher erases the config shape, and
    // this is the information an author wants back — "Config valid" answers a
    // question nobody asked, and cannot distinguish a rule set from an empty
    // one, which for a gate is the difference that matters.
    let rule_count = config.matchers.len();
    let has_fallback = config.on_no_match.is_some();
    let mut inputs: Vec<String> = config
        .matchers
        .iter()
        .flat_map(|fm| collect_input_urls(&fm.predicate))
        .collect();
    inputs.sort_unstable();
    inputs.dedup();

    match domain {
        Domain::Test => {
            let registry = build_test_registry();
            registry
                .load_matcher(config)
                .map_err(|e| format!("config invalid: {e}"))?;
        }
        Domain::Http => {
            let registry = build_http_registry();
            registry
                .load_matcher(config)
                .map_err(|e| format!("config invalid: {e}"))?;
        }
        Domain::Claude => {
            let registry = build_claude_registry();
            registry
                .load_matcher(config)
                .map_err(|e| format!("config invalid: {e}"))?;
        }
    }

    println!("Config valid");
    println!("  rules:     {rule_count}");
    println!(
        "  inputs:    {}",
        if inputs.is_empty() {
            "(none)".to_string()
        } else {
            inputs.join(", ")
        }
    );
    println!(
        "  fallback:  {}",
        if has_fallback {
            "yes"
        } else {
            "none — an unmatched context returns no action"
        }
    );
    if rule_count == 0 {
        println!("\n  Note: zero rules. Every context will fall through.");
    }
    Ok(())
}

/// Collect the input type URLs a predicate tree references, recursively.
fn collect_input_urls(predicate: &rumi::PredicateConfig) -> Vec<String> {
    use rumi::PredicateConfig as P;
    match predicate {
        P::Single(single) => vec![single.input.type_url.clone()],
        P::And { predicates } | P::Or { predicates } => {
            predicates.iter().flat_map(collect_input_urls).collect()
        }
        P::Not { predicate } => collect_input_urls(predicate),
    }
}

#[allow(clippy::unnecessary_wraps)]
/// `rumi --skill [-r NAME]` — emit agent-facing documentation.
///
/// Type URL tables come from the live registries, so the document cannot
/// describe an input that is not actually registered.
fn cmd_skill(args: &[String]) -> Result<(), String> {
    if let Some(pos) = args.iter().position(|a| a == "-r" || a == "--reference") {
        let name = args.get(pos + 1).ok_or_else(|| {
            format!(
                "--skill -r requires a reference name (one of: {})",
                skill::REFERENCE_NAMES.join(", ")
            )
        })?;
        let body = skill::reference(name).ok_or_else(|| {
            format!(
                "unknown reference \"{name}\" (one of: {})",
                skill::REFERENCE_NAMES.join(", ")
            )
        })?;
        print!("{body}");
        return Ok(());
    }

    let test = build_test_registry();
    let http = build_http_registry();
    let claude = build_claude_registry();

    print!(
        "{}",
        skill::skill(
            &test.input_type_urls(),
            &test.matcher_type_urls(),
            &http.input_type_urls(),
            &claude.input_type_urls(),
        )
    );
    Ok(())
}

fn cmd_info(args: &[String]) -> Result<(), String> {
    let (domain, rest) = detect_domain(args);
    let verbose = rest.iter().any(|a| a == "--verbose" || a == "-v");

    match domain {
        Domain::Test => print_registry_info(&build_test_registry(), verbose),
        Domain::Http => print_registry_info(&build_http_registry(), verbose),
        Domain::Claude => print_registry_info(&build_claude_registry(), verbose),
    }

    Ok(())
}

/// Print the registry's type URLs, optionally with what each one is and which
/// config field it takes.
///
/// `--verbose` closes the authoring loop: without it a caller knows a type URL
/// exists but must guess between `name`, `key` and `header` for its config
/// field. The descriptions come from `skill.rs`, the same source `--skill`
/// renders from — two copies of that table diverged within hours the last time.
fn print_registry_info<Ctx: 'static>(registry: &rumi::Registry<Ctx>, verbose: bool) {
    println!("Registered inputs:");
    for url in registry.input_type_urls() {
        if verbose {
            let cfg = skill::config_key(url)
                .map_or_else(|| "  (no config)".to_string(), |k| format!("  config.{k}"));
            let prose = skill::describe(url);
            if prose.is_empty() {
                println!("  {url}{cfg}");
            } else {
                println!("  {url}{cfg}\n      {prose}");
            }
        } else {
            println!("  {url}");
        }
    }

    println!("\nRegistered matchers:");
    for url in registry.matcher_type_urls() {
        if verbose {
            let prose = skill::describe(url);
            if prose.is_empty() {
                println!("  {url}");
            } else {
                println!("  {url}\n      {prose}");
            }
        } else {
            println!("  {url}");
        }
    }

    if !verbose {
        println!("\nRe-run with --verbose for config field names.");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// run: test domain (default)
// ═══════════════════════════════════════════════════════════════════════════════

/// True if `--trace` appears anywhere in the args.
fn wants_trace(args: &[String]) -> bool {
    args.iter().any(|a| a == "--trace")
}

/// Strip `--trace` so domain arg parsers never see it.
fn without_trace(args: &[String]) -> Vec<String> {
    args.iter().filter(|a| *a != "--trace").cloned().collect()
}

/// Evaluate, printing either the action or the full trace.
fn report<Ctx, A: std::fmt::Display + Clone + Send + Sync + 'static>(
    matcher: &rumi::Matcher<Ctx, A>,
    ctx: &Ctx,
    trace: bool,
) {
    if trace {
        trace_output::print(&matcher.evaluate_with_trace(ctx));
    } else {
        match matcher.evaluate(ctx) {
            Some(action) => println!("{action}"),
            None => println!("(no match)"),
        }
    }
}

fn cmd_run_test(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("run requires a config file path".into());
    }

    let trace = wants_trace(args);
    let args = without_trace(args);
    let config_path = &args[0];
    let context = parse_context_pairs(&args[1..])?;

    let config = load_config(config_path)?;
    let registry = build_test_registry();
    let matcher = registry
        .load_matcher(config)
        .map_err(|e| format!("config load failed: {e}"))?;

    let ctx = build_test_context(&context);
    report(&matcher, &ctx, trace);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// run: http domain
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_run_http(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("run http requires a config file path".into());
    }

    let trace = wants_trace(args);
    let args = without_trace(args);
    let config_path = &args[0];
    let http_args = parse_http_args(&args[1..])?;

    let config = load_config(config_path)?;
    let registry = build_http_registry();
    let matcher = registry
        .load_matcher(config)
        .map_err(|e| format!("config load failed: {e}"))?;

    let ctx = build_http_context(&http_args);
    report(&matcher, &ctx, trace);

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// run: claude domain
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_run_claude(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("run claude requires a config file path".into());
    }

    if args.iter().any(|a| a == "--stdin") {
        // Never returns: hook mode answers with an exit code, and mapping a
        // Result back onto one here would lose the distinction between "block"
        // and "errored", which are the same exit code for opposite reasons.
        run_claude_hook(args);
    }

    let trace = wants_trace(args);
    let args = without_trace(args);
    let config_path = &args[0];
    let claude_args = parse_claude_args(&args[1..])?;

    let config = load_config(config_path)?;
    let registry = build_claude_registry();
    let matcher = registry
        .load_matcher(config)
        .map_err(|e| format!("config load failed: {e}"))?;

    let ctx = build_claude_context(&claude_args)?;
    report(&matcher, &ctx, trace);

    Ok(())
}

/// `rumi run claude --stdin <config>` — read a hook payload, exit with a verdict.
///
/// Diverges rather than returning, because the exit code *is* the answer. See
/// `hook.rs` for the contract; the short version is that every failure exits
/// with the block code, because Claude Code lets a tool call proceed on any
/// non-zero code other than 2.
fn run_claude_hook(args: &[String]) -> ! {
    use std::io::Read;

    let args = without_trace(args);
    let config_path = &args[0];

    let deny_actions: Vec<String> = match flag_value(&args, "--deny-action") {
        Some(name) => vec![name],
        None => hook::DEFAULT_DENY_ACTIONS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    };

    let mut raw = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut raw) {
        block("could not read the hook payload from stdin", &e.to_string());
    }

    let payload = match hook::parse_payload(&raw) {
        Ok(p) => p,
        Err(e) => block("malformed hook payload", &e),
    };

    let config = match load_config(config_path) {
        Ok(c) => c,
        Err(e) => block("could not load the hook config", &e),
    };

    let matcher = match build_claude_registry().load_matcher(config) {
        Ok(m) => m,
        Err(e) => block("hook config is invalid", &e.to_string()),
    };

    let claude_args = ClaudeArgs {
        event: payload.event.clone(),
        tool: payload.tool_name.clone(),
        arguments: payload
            .arguments
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        cwd: (!payload.cwd.is_empty()).then(|| payload.cwd.clone()),
        branch: None,
        session_id: (!payload.session_id.is_empty()).then(|| payload.session_id.clone()),
    };

    let ctx = match build_claude_context(&claude_args) {
        Ok(c) => c,
        Err(e) => block("hook payload describes an event rumi does not know", &e),
    };

    let decision = matcher.evaluate(&ctx);
    let code = hook::exit_code_for(decision.as_deref(), &deny_actions);

    if code == hook::EXIT_BLOCK {
        // stderr on exit 2 is shown to the model, so it should say why.
        eprintln!(
            "blocked by rumi: rule matched with action \"{}\"",
            decision.as_deref().unwrap_or("(none)")
        );
    }
    std::process::exit(code);
}

/// Report a failure and exit with the block code.
///
/// Failing closed is the whole point: `rumi` reaching this path means it does
/// not know whether the call is safe.
fn block(what: &str, detail: &str) -> ! {
    eprintln!("blocked by rumi: {what} — {detail}");
    eprintln!("(rumi fails closed: it could not decide, so it did not allow)");
    std::process::exit(hook::EXIT_BLOCK);
}

/// Value of `--flag value`, if present.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Registry assembly (composition root)
// ═══════════════════════════════════════════════════════════════════════════════

fn build_test_registry() -> rumi::Registry<TestContext> {
    let builder = rumi::RegistryBuilder::new();
    rumi_test::register(builder).build()
}

fn build_http_registry() -> rumi::Registry<HttpRequest> {
    let builder = rumi::RegistryBuilder::new();
    rumi_http::register_simple(builder).build()
}

fn build_claude_registry() -> rumi::Registry<HookContext> {
    let builder = rumi::RegistryBuilder::new();
    rumi::claude::register(builder).build()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Context builders
// ═══════════════════════════════════════════════════════════════════════════════

fn build_test_context(pairs: &HashMap<String, String>) -> TestContext {
    let mut ctx = TestContext::new();
    for (k, v) in pairs {
        ctx = ctx.with(k, v);
    }
    ctx
}

fn build_http_context(args: &HttpArgs) -> HttpRequest {
    let mut builder = HttpRequest::builder().method(&args.method).path(&args.path);
    for (k, v) in &args.headers {
        builder = builder.header(k, v);
    }
    for (k, v) in &args.query_params {
        builder = builder.query_param(k, v);
    }
    builder.build()
}

fn build_claude_context(args: &ClaudeArgs) -> Result<HookContext, String> {
    let mut ctx = match args.event.as_str() {
        "PreToolUse" => HookContext::pre_tool_use(&args.tool),
        "PostToolUse" => HookContext::post_tool_use(&args.tool),
        "Stop" => HookContext::stop(),
        "SubagentStop" => HookContext::subagent_stop(),
        "UserPromptSubmit" => HookContext::user_prompt_submit(),
        "SessionStart" => HookContext::session_start(),
        "SessionEnd" => HookContext::session_end(),
        "PreCompact" => HookContext::pre_compact(),
        "Notification" => HookContext::notification(),
        other => {
            return Err(format!(
                "unknown event \"{other}\". Valid events: PreToolUse, PostToolUse, Stop, \
                 SubagentStop, UserPromptSubmit, SessionStart, SessionEnd, PreCompact, Notification"
            ))
        }
    };

    for (k, v) in &args.arguments {
        ctx = ctx.with_arg(k, v);
    }
    if let Some(ref cwd) = args.cwd {
        ctx = ctx.with_cwd(cwd);
    }
    if let Some(ref branch) = args.branch {
        ctx = ctx.with_git_branch(branch);
    }
    if let Some(ref session) = args.session_id {
        ctx = ctx.with_session_id(session);
    }

    Ok(ctx)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Config loading
// ═══════════════════════════════════════════════════════════════════════════════

fn load_config(path: &str) -> Result<MatcherConfig<String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("failed to read \"{path}\": {e}"))?;

    let is_json = std::path::Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));

    if is_json {
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {e}"))
    } else {
        serde_yaml::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Argument parsing
// ═══════════════════════════════════════════════════════════════════════════════

/// Parsed HTTP arguments.
#[derive(Debug, Default)]
struct HttpArgs {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    query_params: Vec<(String, String)>,
}

/// Parsed Claude arguments.
#[derive(Debug, Default)]
struct ClaudeArgs {
    event: String,
    tool: String,
    arguments: Vec<(String, String)>,
    cwd: Option<String>,
    branch: Option<String>,
    session_id: Option<String>,
}

fn parse_http_args(args: &[String]) -> Result<HttpArgs, String> {
    let mut result = HttpArgs::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--method" => {
                i += 1;
                result.method = next_value(args, i, "--method")?;
                i += 1;
            }
            "--path" => {
                i += 1;
                result.path = next_value(args, i, "--path")?;
                i += 1;
            }
            "--header" => {
                i += 1;
                let pair = next_value(args, i, "--header")?;
                let (k, v) = split_kv(&pair, "--header")?;
                result.headers.push((k, v));
                i += 1;
            }
            "--query" => {
                i += 1;
                let pair = next_value(args, i, "--query")?;
                let (k, v) = split_kv(&pair, "--query")?;
                result.query_params.push((k, v));
                i += 1;
            }
            other => return Err(format!("unexpected argument \"{other}\"")),
        }
    }

    if result.method.is_empty() {
        return Err("--method is required for http domain".into());
    }
    if result.path.is_empty() {
        return Err("--path is required for http domain".into());
    }

    Ok(result)
}

fn parse_claude_args(args: &[String]) -> Result<ClaudeArgs, String> {
    let mut result = ClaudeArgs::default();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--event" => {
                i += 1;
                result.event = next_value(args, i, "--event")?;
                i += 1;
            }
            "--tool" => {
                i += 1;
                result.tool = next_value(args, i, "--tool")?;
                i += 1;
            }
            "--arg" => {
                i += 1;
                let pair = next_value(args, i, "--arg")?;
                let (k, v) = split_kv(&pair, "--arg")?;
                result.arguments.push((k, v));
                i += 1;
            }
            "--cwd" => {
                i += 1;
                result.cwd = Some(next_value(args, i, "--cwd")?);
                i += 1;
            }
            "--branch" => {
                i += 1;
                result.branch = Some(next_value(args, i, "--branch")?);
                i += 1;
            }
            "--session" => {
                i += 1;
                result.session_id = Some(next_value(args, i, "--session")?);
                i += 1;
            }
            other => return Err(format!("unexpected argument \"{other}\"")),
        }
    }

    if result.event.is_empty() {
        return Err("--event is required for claude domain".into());
    }

    Ok(result)
}

/// Parse `--context key=value...` pairs (test domain).
fn parse_context_pairs(args: &[String]) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        if args[i] == "--context" {
            i += 1;
            while i < args.len() && !args[i].starts_with("--") {
                let pair = &args[i];
                let (key, value) = pair.split_once('=').ok_or_else(|| {
                    format!("invalid context pair \"{pair}\", expected key=value")
                })?;
                map.insert(key.to_owned(), value.to_owned());
                i += 1;
            }
        } else {
            return Err(format!("unexpected argument \"{}\"", args[i]));
        }
    }

    Ok(map)
}

/// Get the next argument value, erroring if missing.
fn next_value(args: &[String], i: usize, flag: &str) -> Result<String, String> {
    args.get(i)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

/// Split a `key=value` pair.
fn split_kv(pair: &str, flag: &str) -> Result<(String, String), String> {
    pair.split_once('=')
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .ok_or_else(|| format!("invalid {flag} pair \"{pair}\", expected key=value"))
}

fn print_usage() {
    eprintln!(
        "Usage: rumi <command> [domain] [options]

Commands:
  run [http|claude] <config> [flags]    Run config against context
  check [http|claude] <config>          Validate config
  info [http|claude]                    Print registered type URLs
  --skill [-r NAME]                     Print agent-facing documentation
  help, --help, -h                      Show this help

Domains:
  (default)  Test domain (key-value context)
  http       HTTP matching (method, path, headers, query params)
  claude     Claude Code hooks (event, tool, arguments)

Flags (all domains):
  --trace                               Show why each rule did or did not match

Flags (claude domain):
  --stdin                               Read a Claude Code hook payload as JSON
                                        on stdin and answer with an exit code:
                                        0 allow, 2 block. Every failure exits 2,
                                        because Claude Code lets a tool call
                                        proceed on any other non-zero code.
  --deny-action NAME                    Action that means block (default: deny, block)

Flags (test domain, default):
  --context key=value...                Context key-value pairs

Flags (http domain):
  --method METHOD                       HTTP method (required)
  --path PATH                           Request path (required)
  --header key=value                    Header (repeatable)
  --query key=value                     Query parameter (repeatable)

Flags (claude domain):
  --event EVENT                         Hook event name (required)
  --tool NAME                           Tool name
  --arg key=value                       Tool argument (repeatable)
  --cwd PATH                            Working directory
  --branch NAME                         Git branch
  --session ID                          Session ID

Examples:
  rumi run config.yaml --context method=GET
  rumi run http routes.yaml --method GET --path /api/users
  rumi run claude hooks.yaml --event PreToolUse --tool Bash
  rumi check http routes.yaml
  rumi info http"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Domain detection ────────────────────────────────────────────────

    #[test]
    fn detect_domain_http() {
        let args: Vec<String> = vec!["http".into(), "config.yaml".into()];
        let (domain, rest) = detect_domain(&args);
        assert_eq!(domain, Domain::Http);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0], "config.yaml");
    }

    #[test]
    fn detect_domain_claude() {
        let args: Vec<String> = vec!["claude".into(), "hooks.yaml".into()];
        let (domain, rest) = detect_domain(&args);
        assert_eq!(domain, Domain::Claude);
        assert_eq!(rest.len(), 1);
    }

    #[test]
    fn detect_domain_default() {
        let args: Vec<String> = vec!["config.yaml".into(), "--context".into(), "k=v".into()];
        let (domain, rest) = detect_domain(&args);
        assert_eq!(domain, Domain::Test);
        assert_eq!(rest.len(), 3);
    }

    // ─── Registry builders ───────────────────────────────────────────────

    #[test]
    fn build_test_registry_has_expected_types() {
        let registry = build_test_registry();
        assert!(registry.contains_input("xuma.test.v1.StringInput"));
        assert!(registry.contains_matcher("xuma.core.v1.StringMatcher"));
        assert!(registry.contains_matcher("xuma.core.v1.BoolMatcher"));
    }

    #[test]
    fn build_http_registry_has_expected_types() {
        let registry = build_http_registry();
        assert!(registry.contains_input("xuma.http.v1.PathInput"));
        assert!(registry.contains_input("xuma.http.v1.MethodInput"));
        assert!(registry.contains_input("xuma.http.v1.HeaderInput"));
        assert!(registry.contains_input("xuma.http.v1.QueryParamInput"));
        assert!(registry.contains_matcher("xuma.core.v1.StringMatcher"));
    }

    #[test]
    fn build_claude_registry_has_expected_types() {
        let registry = build_claude_registry();
        assert!(registry.contains_input("xuma.claude.v1.EventInput"));
        assert!(registry.contains_input("xuma.claude.v1.ToolNameInput"));
        assert!(registry.contains_input("xuma.claude.v1.ArgumentInput"));
        assert!(registry.contains_input("xuma.claude.v1.SessionIdInput"));
        assert!(registry.contains_input("xuma.claude.v1.CwdInput"));
        assert!(registry.contains_input("xuma.claude.v1.GitBranchInput"));
        assert!(registry.contains_matcher("xuma.core.v1.StringMatcher"));
    }

    // ─── HTTP argument parsing ───────────────────────────────────────────

    #[test]
    fn parse_http_args_basic() {
        let args: Vec<String> = vec![
            "--method".into(),
            "GET".into(),
            "--path".into(),
            "/api/users".into(),
        ];
        let result = parse_http_args(&args).unwrap();
        assert_eq!(result.method, "GET");
        assert_eq!(result.path, "/api/users");
    }

    #[test]
    fn parse_http_args_with_headers_and_query() {
        let args: Vec<String> = vec![
            "--method".into(),
            "POST".into(),
            "--path".into(),
            "/api".into(),
            "--header".into(),
            "content-type=application/json".into(),
            "--header".into(),
            "authorization=Bearer token".into(),
            "--query".into(),
            "page=1".into(),
        ];
        let result = parse_http_args(&args).unwrap();
        assert_eq!(result.method, "POST");
        assert_eq!(result.headers.len(), 2);
        assert_eq!(
            result.headers[0],
            ("content-type".into(), "application/json".into())
        );
        assert_eq!(result.query_params.len(), 1);
        assert_eq!(result.query_params[0], ("page".into(), "1".into()));
    }

    #[test]
    fn parse_http_args_missing_method() {
        let args: Vec<String> = vec!["--path".into(), "/api".into()];
        let result = parse_http_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--method is required"));
    }

    #[test]
    fn parse_http_args_missing_path() {
        let args: Vec<String> = vec!["--method".into(), "GET".into()];
        let result = parse_http_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--path is required"));
    }

    // ─── Claude argument parsing ─────────────────────────────────────────

    #[test]
    fn parse_claude_args_basic() {
        let args: Vec<String> = vec![
            "--event".into(),
            "PreToolUse".into(),
            "--tool".into(),
            "Bash".into(),
        ];
        let result = parse_claude_args(&args).unwrap();
        assert_eq!(result.event, "PreToolUse");
        assert_eq!(result.tool, "Bash");
    }

    #[test]
    fn parse_claude_args_full() {
        let args: Vec<String> = vec![
            "--event".into(),
            "PreToolUse".into(),
            "--tool".into(),
            "Bash".into(),
            "--arg".into(),
            "command=ls -la".into(),
            "--cwd".into(),
            "/home/user".into(),
            "--branch".into(),
            "main".into(),
            "--session".into(),
            "abc-123".into(),
        ];
        let result = parse_claude_args(&args).unwrap();
        assert_eq!(result.event, "PreToolUse");
        assert_eq!(result.tool, "Bash");
        assert_eq!(result.arguments.len(), 1);
        assert_eq!(result.arguments[0], ("command".into(), "ls -la".into()));
        assert_eq!(result.cwd, Some("/home/user".into()));
        assert_eq!(result.branch, Some("main".into()));
        assert_eq!(result.session_id, Some("abc-123".into()));
    }

    #[test]
    fn parse_claude_args_missing_event() {
        let args: Vec<String> = vec!["--tool".into(), "Bash".into()];
        let result = parse_claude_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--event is required"));
    }

    // ─── Context builder ─────────────────────────────────────────────────

    #[test]
    fn build_claude_context_all_events() {
        for event in [
            "PreToolUse",
            "PostToolUse",
            "Stop",
            "SubagentStop",
            "UserPromptSubmit",
            "SessionStart",
            "SessionEnd",
            "PreCompact",
            "Notification",
        ] {
            let args = ClaudeArgs {
                event: event.into(),
                ..Default::default()
            };
            assert!(build_claude_context(&args).is_ok(), "failed for {event}");
        }
    }

    #[test]
    fn build_claude_context_unknown_event() {
        let args = ClaudeArgs {
            event: "BadEvent".into(),
            ..Default::default()
        };
        let err = build_claude_context(&args).unwrap_err();
        assert!(err.contains("unknown event"));
    }

    #[test]
    fn build_http_context_from_args() {
        let args = HttpArgs {
            method: "GET".into(),
            path: "/api/users".into(),
            headers: vec![("content-type".into(), "application/json".into())],
            query_params: vec![("page".into(), "1".into())],
        };
        let ctx = build_http_context(&args);
        assert_eq!(ctx.method(), "GET");
        assert_eq!(ctx.path(), "/api/users");
        assert_eq!(ctx.header("content-type"), Some("application/json"));
        assert_eq!(ctx.query_param("page"), Some("1"));
    }

    // ─── Test domain context parsing ─────────────────────────────────────

    #[test]
    fn parse_context_empty() {
        let result = parse_context_pairs(&[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn parse_context_pairs_basic() {
        let args: Vec<String> = vec!["--context".into(), "method=GET".into(), "path=/api".into()];
        let result = parse_context_pairs(&args).unwrap();
        assert_eq!(result.get("method").unwrap(), "GET");
        assert_eq!(result.get("path").unwrap(), "/api");
    }

    #[test]
    fn parse_context_missing_equals() {
        let args: Vec<String> = vec!["--context".into(), "badformat".into()];
        let result = parse_context_pairs(&args);
        assert!(result.is_err());
    }

    // ─── Config loading ──────────────────────────────────────────────────

    /// Checked-in fixture, resolved relative to this crate so the tests pass
    /// from any working directory and on any machine.
    fn fixture_path() -> String {
        format!("{}/tests/fixtures/config.yaml", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn load_yaml_config() {
        let config = load_config(&fixture_path());
        assert!(
            config.is_ok(),
            "failed to load YAML config: {:?}",
            config.err()
        );
    }

    #[test]
    fn eval_yaml_config() {
        let config = load_config(&fixture_path()).unwrap();
        let registry = build_test_registry();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = TestContext::new().with("method", "GET");
        assert_eq!(matcher.evaluate(&ctx), Some("route-get".to_string()));

        let ctx = TestContext::new().with("method", "POST");
        assert_eq!(matcher.evaluate(&ctx), Some("route-post".to_string()));

        let ctx = TestContext::new().with("method", "DELETE");
        assert_eq!(matcher.evaluate(&ctx), Some("fallback".to_string()));
    }
    // ─── Skill accuracy ──────────────────────────────────────────────────

    /// Every config key named in `skill::CONFIG_KEYS` must be the key the real
    /// loader accepts.
    ///
    /// This test exists because the hand-written half of `skill.rs` claimed
    /// `ArgumentInput` reads `config.key` when the struct field is `name`. The
    /// generated half of that file cannot drift; this half could, and did,
    /// within hours of being written. A doc claim that CI does not check is not
    /// a doc claim, it is a guess.
    #[test]
    fn skill_hints_name_real_config_keys() {
        for (type_url, key) in skill::CONFIG_KEYS {
            let yaml = format!(
                r#"
matchers:
  - predicate:
      type: single
      input: {{ type_url: "{type_url}", config: {{ {key}: "x" }} }}
      value_match: {{ Exact: "y" }}
    on_match: {{ type: action, action: "a" }}
"#
            );
            let config: MatcherConfig<String> =
                serde_yaml::from_str(&yaml).unwrap_or_else(|e| panic!("{type_url}: {e}"));

            let loaded = if type_url.starts_with("xuma.http.") {
                build_http_registry().load_matcher(config).map(|_| ())
            } else if type_url.starts_with("xuma.claude.") {
                build_claude_registry().load_matcher(config).map(|_| ())
            } else {
                build_test_registry().load_matcher(config).map(|_| ())
            };

            assert!(
                loaded.is_ok(),
                "skill.rs documents `{type_url}` as taking config.{key}, but the \
                 loader rejected it: {:?}",
                loaded.err()
            );
        }
    }

    /// A wrong key must actually fail, or the test above proves nothing.
    #[test]
    fn skill_hint_test_would_catch_a_wrong_key() {
        let yaml = r#"
matchers:
  - predicate:
      type: single
      input: { type_url: "xuma.claude.v1.ArgumentInput", config: { key: "cmd" } }
      value_match: { Exact: "y" }
    on_match: { type: action, action: "a" }
"#;
        let parsed = serde_yaml::from_str::<MatcherConfig<String>>(yaml)
            .map(|c| build_claude_registry().load_matcher(c).map(|_| ()));
        assert!(
            matches!(parsed, Err(_) | Ok(Err(_))),
            "the old wrong key `config.key` was accepted — this guard is inert"
        );
    }
}
