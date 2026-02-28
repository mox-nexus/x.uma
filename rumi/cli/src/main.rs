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
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn cmd_info(args: &[String]) -> Result<(), String> {
    let (domain, _) = detect_domain(args);

    match domain {
        Domain::Test => print_registry_info(&build_test_registry()),
        Domain::Http => print_registry_info(&build_http_registry()),
        Domain::Claude => print_registry_info(&build_claude_registry()),
    }

    Ok(())
}

fn print_registry_info<Ctx: 'static>(registry: &rumi::Registry<Ctx>) {
    println!("Registered inputs:");
    for url in registry.input_type_urls() {
        println!("  {url}");
    }

    println!("\nRegistered matchers:");
    for url in registry.matcher_type_urls() {
        println!("  {url}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// run: test domain (default)
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_run_test(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("run requires a config file path".into());
    }

    let config_path = &args[0];
    let context = parse_context_pairs(&args[1..])?;

    let config = load_config(config_path)?;
    let registry = build_test_registry();
    let matcher = registry
        .load_matcher(config)
        .map_err(|e| format!("config load failed: {e}"))?;

    let ctx = build_test_context(&context);
    match matcher.evaluate(&ctx) {
        Some(action) => println!("{action}"),
        None => println!("(no match)"),
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// run: http domain
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_run_http(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("run http requires a config file path".into());
    }

    let config_path = &args[0];
    let http_args = parse_http_args(&args[1..])?;

    let config = load_config(config_path)?;
    let registry = build_http_registry();
    let matcher = registry
        .load_matcher(config)
        .map_err(|e| format!("config load failed: {e}"))?;

    let ctx = build_http_context(&http_args);
    match matcher.evaluate(&ctx) {
        Some(action) => println!("{action}"),
        None => println!("(no match)"),
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// run: claude domain
// ═══════════════════════════════════════════════════════════════════════════════

fn cmd_run_claude(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("run claude requires a config file path".into());
    }

    let config_path = &args[0];
    let claude_args = parse_claude_args(&args[1..])?;

    let config = load_config(config_path)?;
    let registry = build_claude_registry();
    let matcher = registry
        .load_matcher(config)
        .map_err(|e| format!("config load failed: {e}"))?;

    let ctx = build_claude_context(&claude_args)?;
    match matcher.evaluate(&ctx) {
        Some(action) => println!("{action}"),
        None => println!("(no match)"),
    }

    Ok(())
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
  help                                  Show this help

Domains:
  (default)  Test domain (key-value context)
  http       HTTP matching (method, path, headers, query params)
  claude     Claude Code hooks (event, tool, arguments)

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

    #[test]
    fn load_yaml_config() {
        let config = load_config("/tmp/xuma-yaml-test/config.yaml");
        assert!(
            config.is_ok(),
            "failed to load YAML config: {:?}",
            config.err()
        );
    }

    #[test]
    fn eval_yaml_config() {
        let config = load_config("/tmp/xuma-yaml-test/config.yaml").unwrap();
        let registry = build_test_registry();
        let matcher = registry.load_matcher(config).unwrap();

        let ctx = TestContext::new().with("method", "GET");
        assert_eq!(matcher.evaluate(&ctx), Some("route-get".to_string()));

        let ctx = TestContext::new().with("method", "POST");
        assert_eq!(matcher.evaluate(&ctx), Some("route-post".to_string()));

        let ctx = TestContext::new().with("method", "DELETE");
        assert_eq!(matcher.evaluate(&ctx), Some("fallback".to_string()));
    }
}
