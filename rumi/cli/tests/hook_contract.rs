//! End-to-end tests for `rumi run claude --stdin`.
//!
//! These drive the built binary rather than the library, because the thing
//! under test *is* the process exit code — the contract Claude Code consumes.
//! A unit test on `exit_code_for` cannot catch a wiring mistake that makes
//! every payload take the allow path.
//!
//! The property that matters: Claude Code treats exit 2 as "block" and every
//! other non-zero code as a non-blocking error that lets the tool call through.
//! So every failure mode here must exit 2, not 1.

use std::io::Write;
use std::process::{Command, Stdio};

const ALLOW: i32 = 0;
const BLOCK: i32 = 2;

const CONFIG: &str = r#"
matchers:
  - predicate:
      type: and
      predicates:
        - type: single
          input: { type_url: "xuma.claude.v1.EventInput" }
          value_match: { Exact: "PreToolUse" }
        - type: single
          input: { type_url: "xuma.claude.v1.ToolNameInput" }
          value_match: { Exact: "Bash" }
        - type: single
          input: { type_url: "xuma.claude.v1.ArgumentInput", config: { name: "command" } }
          value_match: { Contains: "rm -rf" }
    on_match: { type: action, action: "deny" }
"#;

fn write_config(body: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("rumi-hook-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("cfg-{}.yaml", body.len()));
    std::fs::write(&path, body).unwrap();
    path
}

/// Run the hook with `payload` on stdin, return the exit code.
fn run_hook(config: &std::path::Path, payload: &str) -> i32 {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rumi"))
        .args(["run", "claude"])
        .arg(config)
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn rumi");

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();

    child.wait().unwrap().code().expect("killed by signal")
}

#[test]
fn a_matching_deny_rule_blocks() {
    let cfg = write_config(CONFIG);
    let code = run_hook(
        &cfg,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash",
            "tool_input":{"command":"rm -rf /"}}"#,
    );
    assert_eq!(
        code, BLOCK,
        "the README's front-page demo must actually block"
    );
}

#[test]
fn a_non_matching_call_is_allowed() {
    let cfg = write_config(CONFIG);
    let code = run_hook(
        &cfg,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash",
            "tool_input":{"command":"ls -la"}}"#,
    );
    assert_eq!(code, ALLOW);
}

#[test]
fn a_different_tool_is_allowed() {
    let cfg = write_config(CONFIG);
    let code = run_hook(
        &cfg,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Read",
            "tool_input":{"file_path":"/etc/passwd"}}"#,
    );
    assert_eq!(code, ALLOW, "the rule is scoped to Bash");
}

// ── Fail-closed. The reason this file exists. ───────────────────────────────

#[test]
fn every_failure_mode_blocks_rather_than_allowing() {
    let cfg = write_config(CONFIG);

    let malformed = [
        ("not JSON at all", "definitely not json"),
        ("empty stdin", ""),
        ("a JSON array", "[1,2,3]"),
        ("a bare string", "\"hello\""),
        ("an empty object", "{}"),
        ("no hook_event_name", r#"{"tool_name":"Bash"}"#),
        ("a non-string event", r#"{"hook_event_name":42}"#),
        ("an unknown event", r#"{"hook_event_name":"NotAnEvent"}"#),
    ];

    for (what, payload) in malformed {
        assert_eq!(
            run_hook(&cfg, payload),
            BLOCK,
            "{what}: must fail closed. Exit 1 would let the tool call proceed."
        );
    }
}

#[test]
fn an_unreadable_config_blocks() {
    let missing = std::env::temp_dir().join("rumi-hook-does-not-exist.yaml");
    let code = run_hook(
        &missing,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{}}"#,
    );
    assert_eq!(
        code, BLOCK,
        "a gate that cannot load its rules must not allow"
    );
}

#[test]
fn an_invalid_config_blocks() {
    let cfg = write_config("matchers:\n  - this is not a field matcher\n");
    let code = run_hook(
        &cfg,
        r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{}}"#,
    );
    assert_eq!(code, BLOCK);
}

#[test]
fn the_allow_path_is_reachable_so_these_tests_are_not_inert() {
    // Everything above asserts BLOCK. Without this, a binary that blocked
    // unconditionally would pass the whole file and gate nothing usefully.
    let cfg = write_config(CONFIG);
    assert_eq!(
        run_hook(&cfg, r#"{"hook_event_name":"Stop"}"#),
        ALLOW,
        "an unrelated event must be allowed"
    );
}
